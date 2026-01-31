use crate::prelude::{ConstructLayer, LayerDispatch, Registry};
use keep::*;
use plugmap::DynBuffer;


pub struct EventEmitter<E>
{
    subscribers: DynBuffer<EventSubscriber<E>>,
}


impl<E> EventEmitter<E>
{
    pub fn subscribe(&self) -> Guard<EventSubscriber<E>>
    {
        let keep = Keep::new(EventSubscriber {
            queue: DynBuffer::new(),
        });
        let subscriber = keep;

        let ret = subscriber.read();

        self.subscribers.push_keep(subscriber);

        ret
    }

    pub fn emit(&self, event: E)
    {
        let event = Guard::new(event);

        for sub in self.subscribers.snapshot().as_ref()
        {
            sub.queue.push(event.clone());
        }
    }
}


pub struct EventSubscriber<E>
{
    queue: DynBuffer<Guard<E>>,
}


impl<E> EventSubscriber<E>
{
    pub fn pop(&self) -> Option<Guard<E>>
    {
        self.queue.pop().map(|k| k.read().as_ref().clone())
    }
}


impl<E, D, Err, Res> ConstructLayer<D, Err, Res> for EventEmitter<E>
where
    E: 'static,
    EventEmitter<E>: LayerDispatch<D, Error = Err, Response = Res>,
{
    fn construct(_registry: &Registry<D, Err, Res>) -> Self
    {
        Self {
            subscribers: DynBuffer::new(),
        }
    }
}
