struct VertexInput
{
    @location(0) pos: vec3<f32>,
    @location(1) col: vec3<f32>,
};


struct VertexOutput
{
    @builtin(position) pos: vec4<f32>,
    @location(0) col: vec3<f32>,  
};


@vertex
fn vertex_main(vert: VertexInput) -> VertexOutput
{
    var out: VertexOutput;
    out.col = vert.col;
    out.pos = vec4<f32>(vert.pos, 1.0);
    return out;
}


@fragment
fn fragment_main(vert: VertexOutput) -> @location(0) vec4<f32>
{
    return vec4<f32>(vert.col, 1.0);
}
