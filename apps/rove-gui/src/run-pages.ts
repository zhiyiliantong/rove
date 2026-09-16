import type {Run} from './run-events';

/** Run pages are ordered oldest first. Refresh only the last loaded page, while
 * snapshots/events update older visible runs. Never discard pages on a timer. */
export function merge_run_page(visible:Run[],page:Run[]):Run[]{
  const merged=new Map(visible.map(run=>[run.run_id,run]));
  for(const run of page)merged.set(run.run_id,run);
  return [...merged.values()].sort((a,b)=>{
    const left=`${a.created_at}|${a.run_id}`,right=`${b.created_at}|${b.run_id}`;
    return left<right?-1:left>right?1:0;
  });
}
