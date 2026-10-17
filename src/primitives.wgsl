struct Instance {
    rect: vec4<f32>, color: vec4<f32>, params: vec4<f32>, clip: vec4<f32>,
    movement: vec4<f32>, rotation: vec4<f32>, origin: vec4<f32>, uv: vec4<f32>, extra: vec4<f32>,
}
@group(0) @binding(0) var<storage, read> instances: array<Instance>;
@group(0) @binding(1) var<uniform> screen: vec4<f32>;
@group(0) @binding(2) var atlas: texture_2d<f32>;
@group(0) @binding(3) var atlas_sampler: sampler;
struct VertexOut { @builtin(position) position: vec4<f32>, @location(0) local: vec2<f32>, @location(1) uv: vec2<f32>, @location(2) @interpolate(flat) instance: u32 }
@vertex fn vs(@builtin(vertex_index) vertex: u32, @builtin(instance_index) index:u32)->VertexOut {
    let i=instances[index];
    let corners=array<vec2<f32>,6>(vec2(0.,0.),vec2(1.,0.),vec2(0.,1.),vec2(0.,1.),vec2(1.,0.),vec2(1.,1.));
    let corner=corners[vertex];
    var p=i.rect.xy+corner*i.rect.zw;
    if i.params.x==2. {let low=min(i.rect.xy,i.rect.zw)-vec2(i.params.y);let high=max(i.rect.xy,i.rect.zw)+vec2(i.params.y);p=mix(low,high,corner);}
    if i.params.x==4. {p=i.params.zw; if vertex==0u {p=i.rect.xy;} if vertex==1u || vertex==4u {p=i.rect.zw;} if vertex>=3u {p=i.params.zw;} }
    let local=p;
    let q=(p-i.origin.xy)*i.movement.z;
    let rz=i.rotation.w;
    let v=vec2(q.x*cos(rz)-q.y*sin(rz),q.x*sin(rz)+q.y*cos(rz));
    let rx=i.rotation.x;let ry=i.rotation.y;
    let y=v.y*cos(rx);let z1=v.y*sin(rx);
    let x=v.x*cos(ry)+z1*sin(ry);let z=-v.x*sin(ry)+z1*cos(ry);
    let perspective=i.rotation.z/max(1.,i.rotation.z-z);
    p=vec2(x,y)*perspective+i.origin.xy+i.movement.xy;
    var out:VertexOut;
    out.position=vec4(p.x/screen.x*2.-1.,1.-p.y/screen.y*2.,0.,1.);
    out.local=local;out.uv=mix(i.uv.xy,i.uv.zw,corner);out.instance=index;return out;
}
@fragment fn fs(v:VertexOut)->@location(0) vec4<f32> {
    let i=instances[v.instance];
    if v.position.x<i.clip.x || v.position.y<i.clip.y || v.position.x>=i.clip.z || v.position.y>=i.clip.w {discard;}
    var coverage=1.;
    if i.params.x==0. && i.params.y>0. {
        let p=v.local-i.rect.xy;let d=min(min(p.x,p.y),min(i.rect.z-p.x,i.rect.w-p.y));
        if d>=i.params.y {discard;}
    }
    if i.params.x==1. {
        let p=v.local-(i.rect.xy+i.rect.zw*0.5);let radius=min(i.rect.z,i.rect.w)*0.5-i.params.y*0.5;
        let distance=abs(length(p)-radius)-i.params.y*0.5;
        coverage=1.-smoothstep(-0.5,0.5,distance);
        let angle=atan2(p.y,p.x);let tau=6.28318530718;
        let relative=(angle-i.params.z+tau*2.)%tau;
        if relative>i.params.w {discard;}
    }
    if i.params.x==2. {
        let a=i.rect.xy;let b=i.rect.zw;let p=v.local;
        let t=clamp(dot(p-a,b-a)/max(dot(b-a,b-a),0.00001),0.,1.);
        let distance=length(p-mix(a,b,t))-i.params.y*0.5;
        coverage=1.-smoothstep(-0.5,0.5,distance);
    }
    if i.params.x==3. {coverage=textureSample(atlas,atlas_sampler,v.uv).r;}
    return vec4(i.color.rgb,i.color.a*coverage*i.movement.w);
}
