"""Guard WP-06.2/6.3's opt-in Rust/rayon batch path and its default-off fallback."""

from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
RUST = (ROOT / "crates/hf-godot/src/world_renderer.rs").read_text()
LIB = (ROOT / "crates/hf-godot/src/lib.rs").read_text()
VIEW = (ROOT / "godot/scripts/world_view.gd").read_text()
ENTITY = (ROOT / "godot/scripts/render/entity_layer.gd").read_text()
RUNTIME = (ROOT / "godot/scripts/render/rust_world_renderer.gd").read_text()
PROJECT = (ROOT / "godot/project.godot").read_text()


def require(text: str, needle: str) -> None:
    assert needle in text, needle


def main() -> None:
    require(RUST, "par_iter()")
    require(RUST, "pub const INSTANCE_STRIDE: usize = 8")
    require(RUST, "pub const STATIC_STRIDE: usize = 8")
    require(RUST, "pub const GROUND_STRIDE: usize = 5")
    require(RUST, "pub fn static_instances")
    require(RUST, "pub fn ground_instances")
    require(RUST, "a.denned")
    require(RUST, "is_unfound")
    require(LIB, "fn render_batches(&self, z: i32) -> PackedFloat32Array")
    require(LIB, "world_renderer::instances")
    require(LIB, "fn render_static_batches(&self, z: i32) -> PackedFloat32Array")
    require(LIB, "fn render_ground_batches(&self, z: i32) -> PackedInt32Array")
    require(VIEW, 'ProjectSettings.get_setting("ostrog/render/world_renderer", false)')
    require(VIEW, 'preload("res://scripts/render/rust_world_renderer.gd")')
    require(ENTITY, "if not v.rust_renderer_enabled:")
    require(RUNTIME, "MultiMeshInstance2D")
    require(RUNTIME, "view.sim.render_batches")
    require(RUNTIME, "view.sim.render_static_batches")
    require(RUNTIME, "view.sim.render_ground_batches")
    require(RUNTIME, "_upload_static")
    require(RUNTIME, "_upload_ground")
    require(RUNTIME, "set_instance_transform_2d")
    require(PROJECT, "[ostrog]")
    require(PROJECT, "render/world_renderer=false")
    print("world renderer guard OK")


if __name__ == "__main__":
    main()
