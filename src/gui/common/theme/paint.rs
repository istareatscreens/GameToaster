use eframe::egui::{self, Color32, FontId, Vec2};

pub fn paint_vertical_gradient(
    painter: &egui::Painter,
    rect: egui::Rect,
    top: Color32,
    bottom: Color32,
) {
    use egui::epaint::{Mesh, Vertex};
    let mut mesh = Mesh::default();
    let uv = egui::pos2(0.0, 0.0);
    mesh.vertices.push(Vertex {
        pos: rect.left_top(),
        uv,
        color: top,
    });
    mesh.vertices.push(Vertex {
        pos: rect.right_top(),
        uv,
        color: top,
    });
    mesh.vertices.push(Vertex {
        pos: rect.right_bottom(),
        uv,
        color: bottom,
    });
    mesh.vertices.push(Vertex {
        pos: rect.left_bottom(),
        uv,
        color: bottom,
    });
    mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
    painter.add(egui::Shape::mesh(mesh));
}

pub fn paint_glyph(
    painter: &egui::Painter,
    center: egui::Pos2,
    glyph: &str,
    size: f32,
    color: Color32,
) {
    painter.text(
        center + Vec2::new(0.0, 1.0),
        egui::Align2::CENTER_CENTER,
        glyph,
        FontId::proportional(size),
        color,
    );
}
