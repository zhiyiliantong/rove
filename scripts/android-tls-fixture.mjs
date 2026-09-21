// Loopback-only, credential-free provider fixture for Android TLS acceptance.
// Use ADB reverse for these ports; never install this self-signed CA on Android.
import http from 'node:http';
import https from 'node:https';
import {readFileSync} from 'node:fs';
const [cert,key]=process.argv.slice(2);
if(!cert||!key)throw Error('Usage: node scripts/android-tls-fixture.mjs <cert.pem> <key.pem>');
const stats={http_requests:0,https_requests:0,tls_rejections:0};
function handle(secure,req,res){
  if(req.url==='/fixture-status'){res.setHeader('content-type','application/json');res.end(JSON.stringify(stats));return;}
  stats[secure?'https_requests':'http_requests']++;
  // Do not record headers, request bodies or credentials.
  req.resume();
  if(req.url.endsWith('/models')){
    res.setHeader('content-type','application/json');res.end(JSON.stringify({data:[{id:'fixture-chat'}]}));
  }else if(req.url.endsWith('/chat/completions')){
    res.setHeader('content-type','text/event-stream');
    res.end(`data: ${JSON.stringify({id:'fixture',object:'chat.completion.chunk',created:1,model:'fixture-chat',choices:[{index:0,delta:{role:'assistant',content:'OK'},finish_reason:null}]})}\n\ndata: [DONE]\n\n`);
  }else{res.writeHead(404);res.end();}
}
const plain=http.createServer((...args)=>handle(false,...args));
const tls=https.createServer({cert:readFileSync(cert),key:readFileSync(key)},(...args)=>handle(true,...args));
tls.on('tlsClientError',()=>stats.tls_rejections++);
plain.listen(19443,'127.0.0.1');tls.listen(19444,'127.0.0.1');
console.log('Credential-free HTTP/TLS fixture on loopback 19443/19444');
function stop(){plain.close();tls.close();plain.closeAllConnections();tls.closeAllConnections();}
process.on('SIGTERM',stop);process.on('SIGINT',stop);
