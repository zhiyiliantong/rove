export interface Run {run_id:string;session_id:string;created_at:string;status:string;input_message:string;error:null|{message:string}}
export interface Snapshot {run:Run;output_tail:string;output_truncated:boolean;snapshot_seq:number}
export interface RunEvent {run_id:string;seq:number;kind:string;data:{text?:string;status?:string}}
export interface EventBatch {events:RunEvent[];last_seq:number;terminal:boolean}

/** Apply atomically so a bad batch never advances the displayed watermark. */
export function apply_events(snapshot:Snapshot,batch:EventBatch):Snapshot {
  let next={...snapshot,run:{...snapshot.run}};
  for(const event of batch.events){
    if(event.run_id!==next.run.run_id)throw new Error('事件所属作业不匹配');
    if(!Number.isSafeInteger(event.seq))throw new Error('事件序号超出客户端范围');
    if(event.seq<=next.snapshot_seq)continue;
    if(event.seq!==next.snapshot_seq+1)throw new Error('事件有缺口，需要恢复快照');
    if(event.kind==='assistant_delta'||event.kind==='tool_output'){
      const chars=Array.from(next.output_tail+(event.data.text??''));
      next.output_truncated ||= chars.length>65536;
      next.output_tail=chars.slice(-65536).join('');
    }
    if(event.kind==='status'&&event.data.status)next.run.status=event.data.status;
    next.snapshot_seq=event.seq;
  }
  if(!Number.isSafeInteger(batch.last_seq)||batch.last_seq>next.snapshot_seq)throw new Error('事件批次水位不匹配');
  return next;
}
