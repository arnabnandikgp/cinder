// Shared Node/browser test assertion, never a production channel primitive.
/** Observe actual termination. The harness deadline must fail, not stand in
 * for an iterator rejection or done: true result. An emitted page also fails. */
export async function expectStreamClosed<T>(iterator:AsyncIterator<T>,label:string,deadlineMs=6000):Promise<void>{
  let timer:ReturnType<typeof setTimeout>|undefined;
  try{
    const closed=await Promise.race([
      Promise.resolve().then(()=>iterator.next()).then(result=>result.done===true,()=>true),
      new Promise<never>((_,reject)=>{timer=setTimeout(()=>reject(Error(label+': stream closure timed out')),deadlineMs);}),
    ]);
    if(!closed)throw Error(label+': unexpected private update');
  }finally{clearTimeout(timer);}
}
