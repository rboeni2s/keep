use std::marker::PhantomData;

use crate::prelude::{ConstructLayer, LayerDispatch, Registry};
use keep::prelude::*;
use plugmap::RingBuffer;


pub struct EventEmitter<E, Err = (), Res = ()>
{
    subscribers: RingBuffer<EventSubscriber<E>>, // TODO: change this to something else wtf, this cannot be a ringbuffer
    _phantom: PhantomData<(Err, Res)>,
}


impl<E> EventEmitter<E>
{
    pub fn subscribe(&self) -> Guard<EventSubscriber<E>>
    {
        let subscriber = Guard::new(EventSubscriber {
            queue: RingBuffer::with_hint(256), // There can only be 256 subscribers to a event
        });

        self.subscribers.push(subscriber.clone());
        subscriber
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
    queue: RingBuffer<Guard<E>>,
}


impl<E> EventSubscriber<E>
{
    pub fn pop(&self) -> Option<Guard<E>>
    {
        self.queue.pop().map(|k| k.read().as_ref().clone())
    }
}


impl<E, D, Err, Res> ConstructLayer<D, Err, Res> for EventEmitter<E, Err, Res>
where
    E: 'static,
    Err: 'static,
    Res: 'static,
    EventEmitter<E, Err, Res>: LayerDispatch<D, Error = Err, Response = Res>,
{
    fn construct(_registry: &Registry<D, Err, Res>) -> Self
    {
        Self {
            subscribers: RingBuffer::with_hint(64), // 64 buffered events
            _phantom: PhantomData,
        }
    }
}
