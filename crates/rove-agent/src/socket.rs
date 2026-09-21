use crate::Agent;
use rove_protocol::{
    ApiError, Request, Response,
    contract::AGENT,
    frame::{read_frame, write_frame},
};
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub async fn serve_connection(
    agent: Arc<Agent>,
    stream: impl tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
) -> anyhow::Result<()> {
    let (mut reader, mut writer) = tokio::io::split(stream);
    let (in_tx, mut incoming) = mpsc::channel(64);
    let (outgoing, mut out_rx) = mpsc::channel::<(Value, Option<Uuid>)>(128);
    let mut tasks = tokio::task::JoinSet::new();
    // Dedicated reader: never drop a partially consumed read_frame future in
    // select! when a response becomes writable.
    tasks.spawn(async move {
        loop {
            match read_frame(&mut reader).await {
                Ok(Some(frame)) => {
                    if in_tx.send(Ok(frame)).await.is_err() {
                        break;
                    }
                }
                Ok(None) => break,
                Err(error) => {
                    let _ = in_tx.send(Err(error)).await;
                    break;
                }
            }
        }
    });
    let mut pending = HashSet::new();
    let mut subscriptions = HashMap::<Uuid, (CancellationToken, Uuid)>::new();
    loop {
        tokio::select! {
            output=out_rx.recv()=>{
                let Some((frame,correlation))=output else{break};
                tokio::time::timeout(std::time::Duration::from_secs(5),write_frame(&mut writer,&frame)).await??;
                if let Some(id)=correlation{
                    pending.remove(&id);
                    if frame["status_code"].as_u64().is_some_and(|status|status!=200) {
                        subscriptions.retain(|_,(_,request_id)|*request_id!=id);
                    }
                }
                if frame["kind"]=="stream_end" && let Some(id)=frame["subscription_id"].as_str().and_then(|s|s.parse().ok()){subscriptions.remove(&id);}
            },
            input=incoming.recv()=>{
                let Some(frame)=input else{break};let frame=frame?;
                let correlation=frame["correlation_id"].as_str().and_then(|s|s.parse::<Uuid>().ok()).ok_or_else(||anyhow::anyhow!("Missing valid correlation_id"))?;
                let error=if pending.contains(&correlation){Some(ApiError::new(409,"correlation_conflict","correlation_id is already active on this connection"))}else if pending.len()>=64{Some(ApiError::new(429,"rate_limited","Too many outstanding requests"))}else{None};
                if let Some(error)=error {
                    write_frame(&mut writer,&json!(Response{kind:"response".into(),correlation_id:correlation,status_code:error.status,body:Some(error.body())})).await?;continue;
                }
                let schema=if frame["kind"]=="unsubscribe"{"SocketUnsubscribe"}else{"SocketRequest"};
                if let Err(error)=AGENT.validate(schema,&frame){write_frame(&mut writer,&json!(Response{kind:"response".into(),correlation_id:correlation,status_code:error.status,body:Some(error.body())})).await?;continue;}
                if schema=="SocketUnsubscribe" {
                    let subscription:Uuid=frame["subscription_id"].as_str().unwrap().parse().unwrap();
                    // This map is connection-local; foreign IDs cannot cancel a
                    // different client's subscription (or any run).
                    if let Some((cancel,_))=subscriptions.remove(&subscription){cancel.cancel();}
                    write_frame(&mut writer,&json!(Response{kind:"response".into(),correlation_id:correlation,status_code:204,body:None})).await?;continue;
                }
                #[allow(unused_mut)]
                let mut request:Request=serde_json::from_value(frame)?;
                pending.insert(correlation);
                #[cfg(all(feature = "easytier", target_os = "linux"))]
                if request.operation_id=="subscribe_run_events" && let Some(target)=request.target.clone() {
                    if target.device_id==agent.store.device_id.0 && agent.network(&target.network_id.to_string()).is_ok_and(|(record,_)|record["state"]=="running") {
                        request.target=None;
                    } else {
                        let validation=AGENT.request(&request).map(|_|());
                        if let Err(error)=validation {
                            write_frame(&mut writer,&json!(Response::error(&request,error))).await?;pending.remove(&correlation);continue;
                        }
                        if subscriptions.len()>=64 {
                            write_frame(&mut writer,&json!(Response::error(&request,ApiError::new(429,"rate_limited","Too many active subscriptions")))).await?;pending.remove(&correlation);continue;
                        }
                        let subscription=Uuid::new_v4();let cancel=CancellationToken::new();
                        subscriptions.insert(subscription,(cancel.clone(),correlation));
                        let tx=outgoing.clone();let agent=agent.clone();
                        tasks.spawn(async move {
                            let id:Uuid=request.path_parameters["run_id"].parse().unwrap();
                            let after=request.query_parameters.get("after_seq").and_then(Value::as_i64).unwrap_or(0);
                            let opened=async {
                                let client=agent.target_client(&target).await?;
                                client.subscribe(id,after).await.map_err(|error|error.downcast::<ApiError>().unwrap_or_else(|_|ApiError::new(503,"target_unreachable","Remote event subscription failed; fetch the target snapshot before resuming")))
                            }.await;
                            let mut stream=match opened {
                                Ok(stream)=>stream,
                                Err(error)=>{let _=tx.send((json!(Response::error(&request,error)),Some(correlation))).await;return;}
                            };
                            if stream.terminal {let _=tx.send((json!(Response::new(&request,204,None)),Some(correlation))).await;return;}
                            let response=Response::new(&request,200,Some(json!({"subscription_id":subscription,"run_id":id})));
                            if tx.send((json!(response),Some(correlation))).await.is_err(){return;}
                            let(reason,error)=loop {
                                tokio::select! {
                                    _=cancel.cancelled()=>break("unsubscribed",None),
                                    result=stream.next()=>match result {
                                        Ok(Some(event))=>if tx.send((json!({"kind":"event","subscription_id":subscription,"event":event}),None)).await.is_err(){return;},
                                        Ok(None)=>break("terminal",None),
                                        Err(_)=>break("transport_error",Some(ApiError::new(503,"target_unreachable","Remote event connection ended; resume from the last sequence or fetch a snapshot"))),
                                    }
                                }
                            };
                            let _=tx.send((json!({"kind":"stream_end","subscription_id":subscription,"reason":reason,"error":error}),None)).await;
                        });
                        continue;
                    }
                }
                if request.operation_id=="subscribe_run_events" && request.target.is_none() {
                    let validation=AGENT.request(&request).map(|_|());
                    let id=request.path_parameters.get("run_id").cloned().unwrap_or_default();
                    let after=request.query_parameters.get("after_seq").and_then(Value::as_i64).unwrap_or(0);
                    let response=match validation {
                        Err(error)=>Some(Response::error(&request,error)),
                        Ok(_) if subscriptions.len()>=64=>Some(Response::error(&request,ApiError::new(429,"rate_limited","Too many active subscriptions"))),
                        Ok(_)=>None,
                    };
                    if let Some(response)=response {write_frame(&mut writer,&json!(response)).await?;pending.remove(&correlation);continue;}
                    let subscription=Uuid::new_v4();let cancel=CancellationToken::new();subscriptions.insert(subscription,(cancel.clone(),correlation));
                    let tx=outgoing.clone();let agent=agent.clone();
                    tasks.spawn(async move {
                        let initial=tokio::select!{_=cancel.cancelled()=>return,value=agent.read_events(&id,after)=>value};
                        let initial=match initial{
                            Err(error)=>{let _=tx.send((json!(Response::error(&request,error)),Some(correlation))).await;return;},
                            Ok((events,true)) if events.is_empty()=>{let _=tx.send((json!(Response::new(&request,204,None)),Some(correlation))).await;return;},
                            Ok(value)=>value,
                        };
                        let opened=Response::new(&request,200,Some(json!({"subscription_id":subscription,"run_id":id})));
                        if tx.send((json!(opened),Some(correlation))).await.is_err(){return;}
                        let mut after=after;
                        let mut pending=Some(initial);
                        let(reason,error)=loop {
                            if cancel.is_cancelled(){break("unsubscribed",None);}
                            let next=if let Some(value)=pending.take(){Ok(value)}else{tokio::select!{_=cancel.cancelled()=>break("unsubscribed",None),value=agent.read_events(&id,after)=>value}};
                            let(events,done)=match next{Ok(v)=>v,Err(e)=>break("transport_error",Some(e))};
                            for event in events {
                                after=event["seq"].as_i64().unwrap();
                                if tx.send((json!({"kind":"event","subscription_id":subscription,"event":event}),None)).await.is_err(){return;}
                            }
                            if done{break("terminal",None);}
                            tokio::select!{_=cancel.cancelled()=>{},_=tokio::time::sleep(std::time::Duration::from_millis(100))=>{}}
                        };
                        let _=tx.send((json!({"kind":"stream_end","subscription_id":subscription,"reason":reason,"error":error}),None)).await;
                    });
                }else{
                    let tx=outgoing.clone();let agent=agent.clone();
                    tasks.spawn(async move{let response=agent.handle(&request).await;let _=tx.send((json!(response),Some(correlation))).await;});
                }
            },
            _=tasks.join_next(),if !tasks.is_empty()=>{},
        }
    }
    // JoinSet drops/aborts only transport tasks, not the scheduler's runs.
    Ok(())
}
