use rove_protocol::{Request, SocketTarget};
use serde_json::{Value, json};
use std::io::{IsTerminal, Write};
use uuid::Uuid;

fn prompt(label: &str, secret: bool) -> anyhow::Result<String> {
    if secret {
        return Ok(rpassword::prompt_password(label)?);
    }
    eprint!("{label}");
    std::io::stderr().flush()?;
    let mut input = String::new();
    anyhow::ensure!(std::io::stdin().read_line(&mut input)? > 0, "Input closed");
    anyhow::ensure!(input.len() <= 65536, "Input is too long");
    Ok(input.trim().to_owned())
}
fn id(label: &str) -> anyhow::Result<Uuid> {
    Ok(prompt(label, false)?.parse()?)
}

pub async fn run(
    client: &rove_sdk::LocalClient,
    mut target: Option<SocketTarget>,
    json_output: bool,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        std::io::stdin().is_terminal() && std::io::stderr().is_terminal(),
        "Interactive mode needs a terminal; use 'rove --help' and an explicit subcommand in scripts"
    );
    anyhow::ensure!(!json_output, "Use explicit subcommands with --json");
    loop {
        super::show_target(target.as_ref());
        eprintln!(
            "\nRove · 漫游者\n1 设备状态  2 网络列表  3 创建网络  4 URL 加入\n5 手动加入  6 模型配置  7 并发设置  8 会话列表\n9 新会话   10 提交任务  11 查看作业  12 取消作业\n13 服务列表  14 选择执行设备  15 分享网络  16 通用 API\n0 退出（后台作业继续）"
        );
        let choice = prompt("选择：", false)?;
        if choice == "0" {
            return Ok(());
        }
        let result: anyhow::Result<()> = async {
            if choice=="14" {
                let network=prompt("网络 ID（留空回到本机）：",false)?;
                target=if network.is_empty(){None}else{Some(SocketTarget {network_id:network.parse()?,device_id:id("设备 ID：")?})};
                return Ok(());
            }
            let mut request=match choice.as_str() {
                "1"=>Request::new("get_device"),
                "2"=>Request::new("list_networks"),
                "3"=>Request::new("create_network").with_body(json!({"display_name":prompt("网络名称：",false)?})),
                "4"=>Request::new("import_network").with_body(json!({"source":"url","url":prompt("分享 URL（隐藏输入）：",true)?})),
                "5"=>{
                    let config: Value=serde_json::from_str(&prompt("完整 JoinConfig JSON（隐藏输入）：",true)?).map_err(|_|anyhow::anyhow!("Invalid JSON"))?;
                    Request::new("import_network").with_body(json!({"source":"manual","config":config}))
                },
                "6"=>{
                    let provider=prompt("提供方（默认 openai_compatible）：",false)?;
                    let base_url=prompt("模型接口 URL：",false)?;
                    let model=prompt("模型名称：",false)?;
                    let key=prompt("API key（隐藏输入，留空表示无需密钥）：",true)?;
                    let key=if key.is_empty(){Value::Null}else{json!(key)};
                    Request::new("set_model_config").with_body(json!({"provider":if provider.is_empty(){"openai_compatible"}else{&provider},"base_url":base_url,"model":model,"api_key":key}))
                },
                "7"=>Request::new("update_settings").with_body(json!({"max_active_runs":prompt("最大并发作业数：",false)?.parse::<u64>()?})),
                "8"=>Request::new("list_sessions"),
                "9"=>Request::new("create_session").with_body(json!({"title":prompt("会话名称：",false)?})),
                "10"=>{
                    let session=id("会话 ID：")?;
                    let message=prompt("告诉目标设备要做什么：",false)?;
                    let request_id=Uuid::new_v4();
                    eprintln!("request_id: {request_id}（响应丢失时重试请保留此 ID）");
                    Request::new("submit_run").with_path("session_id",session).with_body(json!({"request_id":request_id,"message":message}))
                },
                "11"=>Request::new("get_run").with_path("run_id",id("作业 ID：")?),
                "12"=>Request::new("cancel_run").with_path("run_id",id("要取消的作业 ID：")?),
                "13"=>{
                    let mut request=Request::new("list_services");
                    request.query_parameters.insert("network_id".into(),json!(id("网络 ID：")?));request
                },
                "15"=>Request::new("create_network_share").with_path("network_id",id("要分享的网络 ID：")?).with_body(json!({})),
                "16"=>generic()?,
                _=>anyhow::bail!("请选择菜单中的编号"),
            };
            request.target=target.clone();
            let body=super::execute(client,request).await?;
            if let Some(body)=body {
                println!("{}",serde_json::to_string_pretty(&body)?);
                if choice=="15" {eprintln!("{}",rove_sdk::ShareQr::new(body["url"].as_str().unwrap())?.terminal());}
            }
            Ok(())
        }.await;
        if let Err(error) = result {
            eprintln!("rove: {error}");
        }
    }
}
fn generic() -> anyhow::Result<Request> {
    let contract = &rove_protocol::contract::AGENT;
    eprintln!(
        "操作：{}",
        contract
            .operations
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    );
    let operation = prompt("operation_id：", false)?;
    anyhow::ensure!(
        contract.operations.contains_key(&operation),
        "Unknown operation"
    );
    let mut request = Request::new(&operation);
    let path = prompt("path_parameters JSON（无则留空）：", false)?;
    if !path.is_empty() {
        request.path_parameters =
            serde_json::from_str(&path).map_err(|_| anyhow::anyhow!("Invalid path parameters"))?;
    }
    let query = prompt("query_parameters JSON（无则留空）：", false)?;
    if !query.is_empty() {
        request.query_parameters = serde_json::from_str(&query)
            .map_err(|_| anyhow::anyhow!("Invalid query parameters"))?;
    }
    let body = prompt(
        "body JSON（隐藏输入，无则留空；必需空对象时输入 {}）：",
        true,
    )?;
    if !body.is_empty() {
        request.body =
            Some(serde_json::from_str(&body).map_err(|_| anyhow::anyhow!("Invalid JSON body"))?);
    }
    Ok(request)
}
