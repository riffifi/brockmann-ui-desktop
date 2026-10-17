use lyon_tessellation::{BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers};
use std::{env, fs, path::PathBuf};
fn main() {
    let names = [
        "dots-three",
        "dots-three-vertical",
        "plus",
        "minus",
        "x",
        "check",
        "caret-up",
        "caret-down",
        "caret-left",
        "caret-right",
        "arrow-up",
        "arrow-down",
        "arrow-left",
        "arrow-right",
        "play",
        "stop",
        "record",
    ];
    let mut output = String::from("pub static MESHES: &[(&[[f32;2]], &[u16])] = &[\n");
    for suffix in ["", "-bold"] {
        for name in names {
            let path = format!("assets/icons/{name}{suffix}.svg");
            println!("cargo:rerun-if-changed={path}");
            let source = fs::read_to_string(path).unwrap();
            let doc = roxmltree::Document::parse(&source).unwrap();
            let mut mesh: VertexBuffers<[f32; 2], u16> = VertexBuffers::new();
            for node in doc.descendants().filter(|n| n.has_tag_name("path")) {
                let path = lyon_svg::path_utils::build_path(
                    lyon_svg::path::Path::builder().with_svg(),
                    node.attribute("d").unwrap(),
                )
                .unwrap();
                FillTessellator::new()
                    .tessellate_path(
                        &path,
                        &FillOptions::default().with_tolerance(0.08),
                        &mut BuffersBuilder::new(&mut mesh, |v: FillVertex| {
                            v.position().to_array()
                        }),
                    )
                    .unwrap();
            }
            output.push_str(&format!("(&{:?}, &{:?}),\n", mesh.vertices, mesh.indices));
        }
    }
    output.push_str("];\n");
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("icons.rs"),
        output,
    )
    .unwrap();
}
