//! Browser webcam frame source via `getUserMedia` (WebAssembly only).
//!
//! Mirrors [`super::native::NativeCamera`] for the web: it requests the camera
//! once (rear-facing on phones), streams into an offscreen `<video>`, and each
//! [`CameraSource::next_frame`] draws the current video frame onto an offscreen
//! `<canvas>` and reads back the pixels as an [`RgbImage`].
//!
//! `getUserMedia` resolves asynchronously and needs a secure context
//! (HTTPS or `localhost`), so `open` returns immediately and frames only start
//! flowing once the user grants permission and the stream connects.

use super::source::CameraSource;
use image::RgbImage;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{
    CanvasRenderingContext2d, HtmlCanvasElement, HtmlVideoElement, MediaStream,
    MediaStreamConstraints,
};

/// A live browser camera, backed by an offscreen video + canvas.
pub struct WebCamera {
    video: HtmlVideoElement,
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
}

impl WebCamera {
    /// Request the camera and begin streaming into an offscreen video element.
    ///
    /// # Errors
    /// Returns a message if the DOM or `mediaDevices` is unavailable.
    pub fn open() -> Result<Self, String> {
        let window = web_sys::window().ok_or("no window")?;
        let document = window.document().ok_or("no document")?;

        let video: HtmlVideoElement = document
            .create_element("video")
            .map_err(|_| "create <video>")?
            .dyn_into()
            .map_err(|_| "cast <video>")?;
        // Muted + inline autoplay so mobile browsers start the stream.
        video.set_muted(true);
        let _ = video.set_attribute("playsinline", "true");
        let _ = video.set_attribute("autoplay", "true");

        let canvas: HtmlCanvasElement = document
            .create_element("canvas")
            .map_err(|_| "create <canvas>")?
            .dyn_into()
            .map_err(|_| "cast <canvas>")?;
        let ctx: CanvasRenderingContext2d = canvas
            .get_context("2d")
            .map_err(|_| "get 2d context")?
            .ok_or("no 2d context")?
            .dyn_into()
            .map_err(|_| "cast 2d context")?;

        // Prefer the rear camera on phones: { video: { facingMode: "environment" } }.
        let facing = js_sys::Object::new();
        let _ = js_sys::Reflect::set(
            &facing,
            &JsValue::from_str("facingMode"),
            &JsValue::from_str("environment"),
        );
        let constraints = MediaStreamConstraints::new();
        let _ = js_sys::Reflect::set(&constraints, &JsValue::from_str("video"), &facing);
        let _ = js_sys::Reflect::set(&constraints, &JsValue::from_str("audio"), &JsValue::FALSE);

        let media_devices = window
            .navigator()
            .media_devices()
            .map_err(|_| "no mediaDevices (needs HTTPS or localhost)")?;
        let promise = media_devices
            .get_user_media_with_constraints(&constraints)
            .map_err(|_| "getUserMedia rejected")?;

        // Attach the stream to the video once permission is granted.
        let video_for_stream = video.clone();
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(stream) = wasm_bindgen_futures::JsFuture::from(promise).await {
                if let Ok(stream) = stream.dyn_into::<MediaStream>() {
                    // Set srcObject via reflection to avoid MediaProvider typing.
                    let _ = js_sys::Reflect::set(
                        &video_for_stream,
                        &JsValue::from_str("srcObject"),
                        &stream,
                    );
                    let _ = video_for_stream.play();
                }
            }
        });

        Ok(Self { video, canvas, ctx })
    }
}

impl CameraSource for WebCamera {
    fn next_frame(&mut self) -> Option<RgbImage> {
        let (w, h) = (self.video.video_width(), self.video.video_height());
        if w == 0 || h == 0 {
            return None; // stream not ready yet
        }
        if self.canvas.width() != w {
            self.canvas.set_width(w);
        }
        if self.canvas.height() != h {
            self.canvas.set_height(h);
        }
        self.ctx
            .draw_image_with_html_video_element(&self.video, 0.0, 0.0)
            .ok()?;
        let image_data = self
            .ctx
            .get_image_data(0.0, 0.0, f64::from(w), f64::from(h))
            .ok()?;
        let rgba = image_data.data().0; // RGBA, row-major
        let mut img = RgbImage::new(w, h);
        for (i, px) in img.pixels_mut().enumerate() {
            let o = i * 4;
            *px = image::Rgb([rgba[o], rgba[o + 1], rgba[o + 2]]);
        }
        Some(img)
    }
}
