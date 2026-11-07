// Internal lifecycle helper. No channel trust/crypto/financial authority.
// Async-generator finally does not run when return() precedes the first next().
export function privateIterable<T>(make:()=>AsyncGenerator<T,void>,close:()=>unknown):AsyncIterable<T>{
  let consumed=false;
  return {[Symbol.asyncIterator](){
    if(consumed){void close();throw Error('Private subscription unavailable');}
    consumed=true;const values=make();
    return {
      next:()=>values.next(),
      async return(){await close();return values.return(undefined);},
      async throw(error:unknown){await close();return values.throw(error);},
    };
  }};
}
