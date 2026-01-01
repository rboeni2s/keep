use crate::{dispatch::NoDispatch, layer_context::LayerContext, registry::Registry};
use std::any::TypeId;


struct Ctx<'a, E, Err, Res>
{
    layer: &'a LayerContext<E, Err, Res>,
    deps: Vec<TypeId>,
}


pub struct Resolver<'a, E = NoDispatch, Err = (), Res = ()>
{
    layers: Vec<Ctx<'a, E, Err, Res>>,
}


impl<'a, E, Err, Res> Resolver<'a, E, Err, Res>
{
    pub fn new() -> Self
    {
        Resolver { layers: Vec::new() }
    }

    pub fn add_ctx(mut self, layer: &'a LayerContext<E, Err, Res>) -> Self
    {
        self.layers.push(Ctx {
            layer,
            deps: layer.deps(),
        });
        self
    }

    pub fn build_reg(mut self) -> Option<Registry<E, Err, Res>>
    {
        let reg = Registry::new();

        loop
        {
            let mut resolved = None;

            for (i, Ctx { deps, .. }) in &mut self.layers.iter_mut().enumerate()
            {
                if deps.is_empty()
                {
                    resolved = Some(i);
                    break;
                }
            }

            match resolved
            {
                Some(index) =>
                {
                    // Remove the resolved layer from the other layers
                    let resolved_layer = self.layers.remove(index).layer;
                    let dep = resolved_layer.id();

                    // Remove the resolved layer from the other layers dependencies
                    for Ctx { deps, .. } in &mut self.layers
                    {
                        deps.retain(|e| *e != dep);
                    }

                    // Add the layer to the reg
                    resolved_layer.insert_into_reg(&reg);
                }

                None =>
                {
                    if self.layers.is_empty()
                    {
                        return Some(reg);
                    }

                    return None;
                }
            }
        }
    }
}


impl<'a, E, Err, Res> Default for Resolver<'a, E, Err, Res>
{
    fn default() -> Self
    {
        Self::new()
    }
}
