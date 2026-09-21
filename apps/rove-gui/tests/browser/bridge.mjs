// IPC boundary fixture only: production code always uses the native SDK bridge.
export async function bridge(page, options = {}) {
  await page.addInitScript(({ live = false, empty = false, loseSubmit = false, os = 'linux', configured = true,linked=false,pendingRemote=false,offline=false,services=false,sync=false,intro=false,verified=true }) => {
    if(!intro&&!localStorage.getItem('rove-gui-introduction-v1'))localStorage.setItem('rove-gui-introduction-v1',JSON.stringify({version:1,completed:true,step:3,reset_pending:false}));
    const sessions = empty ? [] : [{ session_id: 's1', title: '音乐部署', created_at:'2026-09-16T00:00:00Z' }, { session_id: 's2', title: '代码代理', created_at:'2026-09-15T00:00:00Z' }];
    const execution={network_id:'00000000-0000-4000-8000-000000000111',device_id:'00000000-0000-4000-8000-000000000222'};
    if(linked&&sessions.length)sessions[0].execution_target=execution;
    const pending=pendingRemote?[{request:{request_id:'pending-original-id',message:'原请求，不要重复执行',network_id:execution.network_id},created_at:'2026-09-17T00:00:00Z',error:{code:'target_unreachable',message:'unknown outcome'}}]:[];
    const run = { run_id: 'r1', session_id: 's1', input_message: '先说明部署方案', status: live ? 'running' : 'succeeded', created_at: '2026-09-16T00:00:00Z', error: null };
    const snapshot = { run, snapshot_seq: 1, output_tail: '### 部署建议\n\n先检查空间。\n\n```sh\ndf -h\n```\n\n$$\nA = \\pi r^2\n$$', output_truncated: false };
    const calls = [], events = [], submitted = new Map();
    const catalog={connections:configured?[{connection_id:'c1',provider:'openai_compatible',base_url:'http://127.0.0.1:11434/v1',api_key_configured:true}]:[],models:configured?[{model_id:'m1',connection_id:'c1',model:'test-model',name:'日常助手',tested_at:verified?'2026-09-18T00:00:00Z':null}]:[],default_model_id:configured?'m1':null};
    const tested=new Set();
    const test_key=b=>JSON.stringify([b.provider,b.base_url,b.api_key,b.model]);
    window.testBridge = { sessions, calls, events, snapshot, loseSubmit, failEvents: false };
    window.__TAURI_INTERNALS__ = {
      invoke: async (command, args) => {
        if(command==='plugin:opener|open_url'){calls.push({command,...args});if(window.testBridge.failOpen)throw new Error('test browser unavailable');return;}
        if (command === 'agent_events') {
          calls.push({ command, ...args });
          await new Promise(resolve => setTimeout(resolve, 100));
          if (window.testBridge.failEvents) { window.testBridge.failEvents = false; throw new Error('test stream disconnected'); }
          const batch = events.splice(0);
          return { events: batch, last_seq: batch.at(-1)?.seq ?? args.afterSeq, terminal: run.status === 'cancelled' };
        }
        if (command !== 'agent_call') throw new Error(`Unexpected native call: ${command}`);
        const request = structuredClone(args.request); calls.push(request);
        const op = request.operation_id;
        let body;
        switch (op) {
          case 'get_device': body = { device_id: 'local-device', os }; break;
          case 'test_network_peer': body={endpoint:request.body.endpoint,status:request.body.endpoint.startsWith('udp:')?'unsupported':'reachable',latency_ms:2,message:'TCP only'};break;
          case 'create_network': window.testBridge.createdNetwork=request.body;body={network_id:'created-network',state:'stopped'};break;
          case 'get_network_join_config': body={schema_version:1,network_id:'created-network',display_name:window.testBridge.createdNetwork.display_name,easytier:window.testBridge.createdNetwork.config};break;
          case 'import_network': body={created:true,network:{network_id:'imported',state:'stopped'}};break;
          case 'get_settings': body = { max_active_runs: 4, config_server_url: null }; break;
          case 'get_model_config': body = { config: configured ? { provider: 'openai_compatible', base_url: 'http://127.0.0.1:11434/v1', model: 'test-model' } : null }; break;
          case 'get_model_catalog': body=sync&&request.target?{connections:[{connection_id:'remote-c',provider:'openai',base_url:'https://example.invalid',api_key_configured:true}],models:[{model_id:'remote-m',connection_id:'remote-c',model:'remote',name:'远端配置'}],default_model_id:'remote-m'}:catalog;break;
          case 'sync_model_connection': {
            if(!request.body.overwrite)return {status_code:409,body:{error:{code:'model_sync_conflict',message:'target differs'}}};
            body=catalog;break;
          }
          case 'discover_models': body={models:['real-chat','real-fast'],truncated:false};break;
          case 'test_model': {
            if(window.testBridge.failTest)return {status_code:502,body:{error:{code:'model_provider_failed',message:'Test provider rejected request'}}};
            if(request.body.model_id)catalog.models.find(m=>m.model_id===request.body.model_id).tested_at='2026-09-18T00:00:00Z';
            else tested.add(test_key(request.body));
            body={available:true,tested_at:'2026-09-18T00:00:00Z'};break;
          }
          case 'import_models': {
            const first=!catalog.models.length&&!catalog.onboarding_session_id&&!configured;
            const id=`c${catalog.connections.length+1}`;
            catalog.connections.push({connection_id:id,provider:request.body.provider,base_url:request.body.base_url,api_key_configured:!!request.body.api_key});
            const models=request.body.models.map((m,index)=>({...m,connection_id:id,model_id:`${id}-m${index}`,tested_at:tested.has(test_key({...request.body,model:m.model}))?'2026-09-18T00:00:00Z':null}));catalog.models.push(...models);
            if(request.body.set_default)catalog.default_model_id=models[0].model_id;
            if(first){catalog.onboarding_session_id='onboarding';sessions.push({session_id:'onboarding',title:'初始网络的会话',kind:'network_onboarding'});}
            body=catalog;break;
          }
          case 'rename_model': catalog.models.find(m=>m.model_id===request.path_parameters.model_id).name=request.body.name;body=catalog;break;
          case 'set_default_model': catalog.default_model_id=request.body.model_id;body=catalog;break;
          case 'update_model_connection': Object.assign(catalog.connections.find(c=>c.connection_id===request.path_parameters.connection_id),{base_url:request.body.base_url,api_key_configured:!!request.body.api_key});body=catalog;break;
          case 'delete_model_connection': {
            const id=request.path_parameters.connection_id;catalog.connections=catalog.connections.filter(c=>c.connection_id!==id);catalog.models=catalog.models.filter(m=>m.connection_id!==id);if(!catalog.models.some(m=>m.model_id===catalog.default_model_id))catalog.default_model_id=null;body=catalog;break;
          }
          case 'list_session_submissions': body={items:request.path_parameters.session_id==='s1'?pending:[],next_cursor:null};break;
          case 'list_networks': body={items:sync?[{network_id:execution.network_id,display_name:'同步测试网络',state:'running'}]:services?[{network_id:'n1',display_name:'家庭网络'},{network_id:'n2',display_name:'工作网络'}]:[],next_cursor:null};break;
          case 'list_services': body={items:services&&request.query_parameters.network_id==='n1'?[{service_id:'music',name:'音乐',network_id:'n1',state:'published',target_status:'reachable',endpoints:['http://10.20.30.4:8000','tcp://10.20.30.4:8001'],access_info:'fixture-password <script>alert(1)</script>',last_error:null}]:[],next_cursor:null};break;
          case 'list_network_devices': body={items:sync?[{...execution,display_name:'同步目标',os:'linux',arch:'x86_64',state:'online',capabilities:['system_exec'],overlay_addresses:[],last_error:null}]:[],next_cursor:null};break;
          case 'list_messages': body = { items: [], next_cursor: null }; break;
          case 'list_sessions': body = { items: sessions.filter(s=>!!s.archived_at===(request.query_parameters?.archived??false)).sort((a,b)=>{const compare=((a.created_at??'')+a.session_id).localeCompare((b.created_at??'')+b.session_id);return request.query_parameters?.order==='desc'?-compare:compare;}), next_cursor: null }; break;
          case 'get_session': body=sessions.find(s=>s.session_id===request.path_parameters.session_id);break;
          case 'update_session': body=sessions.find(s=>s.session_id===request.path_parameters.session_id);Object.assign(body,request.body);if(request.body.title!==undefined)body.title_source='manual';break;
          case 'archive_session': case 'restore_session': case 'delete_session': {
            const session=sessions.find(s=>s.session_id===request.path_parameters.session_id);
            if(!session)return {status_code:404,body:{error:{code:'session_not_found',message:'missing'}}};
            if(op==='archive_session')session.archived_at??='2026-09-16T00:00:00Z';
            if(op==='restore_session')session.archived_at=null;
            if(op==='delete_session'){
              if(!session.archived_at||(['queued','running','cancelling'].includes(run.status)&&run.session_id===session.session_id))return {status_code:409,body:{error:{code:'session_has_active_runs',message:'active job'}}};
              sessions.splice(sessions.indexOf(session),1);
            }
            return {status_code:204};
          }
          case 'create_session': body = { session_id: `s${sessions.length + 1}`, title: request.body.title, title_source:request.body.auto_title?'default':'manual', created_at:new Date().toISOString(), execution_target:request.body.execution_target }; sessions.push(body); break;
          case 'list_runs': body = { items: request.query_parameters.session_id === 's1' && !empty && (!request.query_parameters.status||request.query_parameters.status===run.status) ? [run] : [], next_cursor: null,...(offline?{sync_error:{code:'target_unreachable',message:'offline'}}:{}) }; break;
          case 'get_run': body = {...snapshot,run:submitted.get(request.path_parameters.run_id)??snapshot.run,...(offline?{sync_error:{code:'target_unreachable',message:'offline'}}:{})}; break;
          case 'cancel_run': run.status = 'cancelled'; body = run; break;
          case 'submit_run': {
            const session=sessions.find(s=>s.session_id===request.path_parameters.session_id);
            if(session?.title_source==='default'){const chars=[...request.body.message.replace(/\s+/g,' ').trim()];session.title=chars.slice(0,32).join('')+(chars.length>32?'…':'');session.title_source='message';}
            const key = request.body.request_id;
            const pending_index=pending.findIndex(p=>p.request.request_id===key);if(pending_index>=0)pending.splice(pending_index,1);
            if (!submitted.has(key)) submitted.set(key, { ...run, run_id: key, session_id: request.path_parameters.session_id, input_message: request.body.message, status: 'succeeded' });
            if (window.testBridge.loseSubmit) { window.testBridge.loseSubmit = false; throw new Error('test response lost'); }
            body = submitted.get(key); break;
          }
          default: throw new Error(`Unexpected operation: ${op}`);
        }
        return { status_code: 200, body: structuredClone(body) };
      },
    };
  }, options);
}
