use iced::window;

const ICON_SIZE: u32 = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconState {
    Idle,
    Active,
}

pub fn window_icon(state: IconState) -> Option<window::Icon> {
    window::icon::from_rgba(icon_rgba(ICON_SIZE, state), ICON_SIZE, ICON_SIZE).ok()
}

pub fn icon_rgba(size: u32, state: IconState) -> Vec<u8> {
    let mut pixels = vec![0; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let color = pixel_color(x as f32, y as f32, size as f32, state);
            let idx = ((y * size + x) * 4) as usize;
            pixels[idx] = color[0];
            pixels[idx + 1] = color[1];
            pixels[idx + 2] = color[2];
            pixels[idx + 3] = color[3];
        }
    }
    pixels
}

fn pixel_color(x: f32, y: f32, size: f32, state: IconState) -> [u8; 4] {
    let scale = size / 64.0;
    let radius = 14.0 * scale;
    let bg = rounded_rect_alpha(x, y, size, size, radius);
    if bg == 0.0 {
        return [0, 0, 0, 0];
    }

    let top = match state {
        IconState::Idle => [55, 75, 135, 210],
        IconState::Active => [39, 132, 86, 222],
    };
    let mut color = blend([18, 21, 33, (235.0 * bg) as u8], top, y / size * 0.22);

    let cage_alpha = rect_stroke_alpha(
        x,
        y,
        13.0 * scale,
        13.0 * scale,
        38.0 * scale,
        38.0 * scale,
        3.0 * scale,
    );
    let cage_color = match state {
        IconState::Idle => [116, 144, 255, (190.0 * cage_alpha) as u8],
        IconState::Active => [92, 232, 139, (220.0 * cage_alpha) as u8],
    };
    color = over(color, cage_color);

    let cursor = cursor_alpha(x, y, scale);
    color = over(color, [244, 247, 255, (245.0 * cursor) as u8]);

    let accent = circle_alpha(x, y, 45.0 * scale, 18.0 * scale, 5.0 * scale);
    let accent_color = match state {
        IconState::Idle => [255, 183, 77, (230.0 * accent) as u8],
        IconState::Active => [102, 255, 153, (245.0 * accent) as u8],
    };
    over(color, accent_color)
}

fn rounded_rect_alpha(x: f32, y: f32, w: f32, h: f32, r: f32) -> f32 {
    let dx = (x - w / 2.0).abs() - (w / 2.0 - r);
    let dy = (y - h / 2.0).abs() - (h / 2.0 - r);
    let outside = dx.max(0.0).hypot(dy.max(0.0));
    smoothstep(1.0, 0.0, outside - r)
}

fn rect_stroke_alpha(
    x: f32,
    y: f32,
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
    width: f32,
) -> f32 {
    let outer = x >= left && x <= right && y >= top && y <= bottom;
    let inner = x >= left + width && x <= right - width && y >= top + width && y <= bottom - width;
    if outer && !inner { 1.0 } else { 0.0 }
}

fn cursor_alpha(x: f32, y: f32, scale: f32) -> f32 {
    let points = [
        (23.0 * scale, 19.0 * scale),
        (23.0 * scale, 44.0 * scale),
        (30.0 * scale, 38.0 * scale),
        (34.0 * scale, 49.0 * scale),
        (40.0 * scale, 47.0 * scale),
        (36.0 * scale, 36.0 * scale),
        (45.0 * scale, 36.0 * scale),
    ];
    if point_in_polygon(x, y, &points) {
        1.0
    } else {
        0.0
    }
}

fn circle_alpha(x: f32, y: f32, cx: f32, cy: f32, r: f32) -> f32 {
    smoothstep(1.0, 0.0, ((x - cx).hypot(y - cy) - r).abs())
}

fn point_in_polygon(x: f32, y: f32, points: &[(f32, f32)]) -> bool {
    let mut inside = false;
    let mut j = points.len() - 1;
    for i in 0..points.len() {
        let (xi, yi) = points[i];
        let (xj, yj) = points[j];
        if ((yi > y) != (yj > y)) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn blend(a: [u8; 4], b: [u8; 4], t: f32) -> [u8; 4] {
    [
        lerp(a[0], b[0], t),
        lerp(a[1], b[1], t),
        lerp(a[2], b[2], t),
        lerp(a[3], b[3], t),
    ]
}

fn over(base: [u8; 4], top: [u8; 4]) -> [u8; 4] {
    let alpha = top[3] as f32 / 255.0;
    [
        lerp(base[0], top[0], alpha),
        lerp(base[1], top[1], alpha),
        lerp(base[2], top[2], alpha),
        base[3].max(top[3]),
    ]
}

fn lerp(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t).round() as u8
}
