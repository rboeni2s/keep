use crate::dispatch::{LayerDispatch, NoDispatch};
use keep::{Guard, Heap};
use plugmap::PlugMap;
use std::any::TypeId;


pub type Layer<T> = Guard<Box<T>>;


pub struct Registry<E = NoDispatch, Err = (), Res = ()>
{
    map: PlugMap<TypeId, Box<dyn LayerDispatch<E, Error = Err, Response = Res>>>,
}


impl<E, Err, Res> Clone for Registry<E, Err, Res>
{
    fn clone(&self) -> Self
    {
        Self {
            map: self.map.clone(),
        }
    }
}


impl<E, Err, Res> Registry<E, Err, Res>
{
    pub fn new() -> Self
    {
        Self {
            map: PlugMap::new(),
        }
    }

    pub fn insert<T>(&self, layer: T)
    where
        T: LayerDispatch<E, Error = Err, Response = Res> + 'static,
    {
        let layer: Heap<Box<dyn LayerDispatch<E, Error = Err, Response = Res>>> =
            unsafe { Heap::from_ptr(Box::into_raw(Box::new(Box::new(layer)))) };

        self.map.insert(TypeId::of::<T>(), layer);
    }

    pub fn get<T>(&self) -> Option<Layer<T>>
    where
        T: LayerDispatch<E, Error = Err, Response = Res> + 'static,
    {
        self.map.get(&TypeId::of::<T>()).map(|f| unsafe {
            std::mem::transmute::<
                Guard<Box<dyn LayerDispatch<E, Error = Err, Response = Res>>>,
                Guard<Box<T>>,
            >(f)
        })
    }

    pub fn get_unchecked<T>(&self) -> Layer<T>
    where
        T: LayerDispatch<E, Error = Err, Response = Res> + 'static,
    {
        self.get::<T>().expect("Layer was not present in map")
    }

    /// Inserts a boxed layer into the registry
    ///
    /// # Safety
    /// The caller must ensure that `type_id` matches the actual TypeId of the
    /// boxed type of `layer`.
    pub unsafe fn insert_by(
        &self,
        layer: Box<dyn LayerDispatch<E, Error = Err, Response = Res> + 'static>,
        type_id: TypeId,
    )
    {
        let layer: Heap<Box<dyn LayerDispatch<E, Error = Err, Response = Res>>> =
            unsafe { Heap::from_ptr(Box::into_raw(Box::new(layer))) };

        self.map.insert(type_id, layer);
    }

    pub fn dispatch(&self, event: &E) -> Vec<Result<Res, Err>>
    {
        let mut results = vec![];

        for layer in &self.map
        {
            results.push(layer.as_ref().as_ref().layer_dispatch(event));
        }

        results
    }
}


impl<E, Err, Resp> Default for Registry<E, Err, Resp>
{
    fn default() -> Self
    {
        Self::new()
    }
}
