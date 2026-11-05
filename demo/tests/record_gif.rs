//! Records `images/demo.gif`, the README's demo, from the demo app, headless:
//!
//! ```sh
//! cargo test -p egui_stack_demo --test record_gif -- --ignored
//! ```
//!
//! A tab is dragged over the band along a leaf's left edge, which shows the zone it would
//! take; the drop splits the leaf, and the row is arranged as a grid.

use std::{fs::File, io::BufWriter, path::Path};

use egui::{Pos2, Rect, pos2};
use egui_kittest::{Harness, kittest::Queryable as _};
use egui_stack::{Node, NodeIndex};
use egui_stack_demo::DemoApp;
use image::RgbaImage;

const SIZE: [f32; 2] = [960.0, 540.0];
/// How long a frame shows, in milliseconds.
const FRAME_MS: u32 = 60;

struct Recorder<'a> {
    harness: Harness<'a, DemoApp>,
    pointer: Pos2,
    /// The frames, each with how many times in a row it was rendered.
    frames: Vec<(RgbaImage, u32)>,
}

impl Recorder<'_> {
    fn frame(&mut self) {
        self.harness.hover_at(self.pointer);
        self.harness.step();
        self.harness.step();
        let image = self.harness.render().expect("a rendered frame");
        match self.frames.last_mut() {
            Some((last, count)) if *last == image => *count += 1,
            _ => self.frames.push((image, 1)),
        }
    }

    fn hold(&mut self, frames: usize) {
        for _ in 0..frames {
            self.frame();
        }
    }

    /// Moves the pointer to `to` in `frames` frames, easing in and out.
    fn move_to(&mut self, to: Pos2, frames: usize) {
        let from = self.pointer;
        for i in 1..=frames {
            let t = i as f32 / frames as f32;
            let eased = t * t * (3.0 - 2.0 * t);
            self.pointer = from.lerp(to, eased);
            self.frame();
        }
    }

    fn press(&mut self) {
        self.harness.drag_at(self.pointer);
        self.frame();
    }

    fn release(&mut self) {
        self.harness.drop_at(self.pointer);
        self.frame();
    }

    fn leaf_of(&self, tab: &str) -> Rect {
        let tree = self.harness.state().dock_state.main_surface();
        let (node, _) = tree.find_tab(&tab.to_owned()).expect("the tab is docked");
        tree[node].rect().expect("a leaf has a rect")
    }
}

#[test]
#[ignore = "writes images/demo.gif; run it with --ignored to record it again"]
fn record_demo_gif() {
    let harness = Harness::builder()
        .with_size(SIZE)
        .with_pixels_per_point(1.0)
        .with_render_cursor(true)
        .wgpu()
        .build_ui_state(|ui, app: &mut DemoApp| app.show(ui), DemoApp::default());
    let mut recorder = Recorder {
        pointer: pos2(SIZE[0] * 0.5, SIZE[1] * 0.6),
        harness,
        frames: Vec::new(),
    };
    recorder.hold(15);

    // Drag `Console` to the band along the left edge of `Preview`'s leaf.
    let console = recorder.harness.get_by_label("Console").rect().center();
    recorder.move_to(console, 12);
    recorder.press();
    let preview = recorder.leaf_of("Preview");
    let in_band = pos2(preview.left() + preview.width() * 0.1, preview.center().y);
    recorder.move_to(pos2(console.x, console.y + 60.0), 6);
    recorder.move_to(preview.center(), 16);
    recorder.hold(8);
    recorder.move_to(in_band, 10);
    recorder.hold(18);
    recorder.release();
    recorder.hold(20);

    let root = &recorder.harness.state().dock_state.main_surface()[NodeIndex::root()];
    assert_eq!(
        root.children().len(),
        4,
        "the drop made a fourth leaf in the row"
    );

    // Focus the new leaf, then arrange its row as a grid.
    let console = recorder.harness.get_by_label("Console").rect().center();
    recorder.move_to(console, 12);
    recorder.press();
    recorder.release();
    recorder.hold(4);
    assert!(
        recorder.harness.state().dock_state.focused_leaf().is_some(),
        "a click on a tab focuses its leaf"
    );
    let grid = recorder.harness.get_by_label("Grid").rect().center();
    recorder.move_to(grid, 18);
    recorder.press();
    recorder.release();
    recorder.move_to(pos2(grid.x + 40.0, grid.y + 120.0), 10);
    recorder.hold(40);

    let root = &recorder.harness.state().dock_state.main_surface()[NodeIndex::root()];
    assert!(matches!(root, Node::Grid(_)), "the row became a grid");

    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../images/demo.gif");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let file = BufWriter::new(File::create(&path).expect("images/demo.gif can be written"));
    let mut encoder = gif::Encoder::new(file, SIZE[0] as u16, SIZE[1] as u16, &[]).unwrap();
    encoder.set_repeat(gif::Repeat::Infinite).unwrap();

    // After the first frame, each frame holds only the rectangle that changed, over the
    // previous one, which the GIF keeps.
    let mut previous: Option<RgbaImage> = None;
    for (image, count) in recorder.frames {
        let (x, y, width, height) = match &previous {
            None => (0, 0, image.width(), image.height()),
            Some(previous) => changed_rect(previous, &image).expect("frames in a row differ"),
        };
        let mut part = image::imageops::crop_imm(&image, x, y, width, height)
            .to_image()
            .into_raw();
        let mut frame = gif::Frame::from_rgba_speed(width as u16, height as u16, &mut part, 10);
        frame.left = x as u16;
        frame.top = y as u16;
        frame.delay = (FRAME_MS * count / 10) as u16;
        frame.dispose = gif::DisposalMethod::Keep;
        encoder.write_frame(&frame).expect("the GIF is written");
        previous = Some(image);
    }
}

/// The smallest rectangle, `(x, y, width, height)`, holding every pixel that differs between
/// `a` and `b`, or `None` if they are the same.
fn changed_rect(a: &RgbaImage, b: &RgbaImage) -> Option<(u32, u32, u32, u32)> {
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (u32::MAX, u32::MAX, 0, 0);
    for (x, y, pixel) in b.enumerate_pixels() {
        if a.get_pixel(x, y) != pixel {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }
    (min_x <= max_x).then(|| (min_x, min_y, max_x - min_x + 1, max_y - min_y + 1))
}
