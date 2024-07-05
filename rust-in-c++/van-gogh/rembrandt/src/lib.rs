use self::bridge::*;
use image::{buffer::ConvertBuffer, imageops::huerotate, GrayImage, ImageBuffer, Rgba};

type ImageRgbaRef<'b> = ImageBuffer<Rgba<u8>, &'b [u8]>;

fn create_image_ref<'b>(buf: &'b [u8], width: u32, height: u32) -> bridge::RbImageRgbaRef<'b> {
    RbImageRgbaRef {
        data: buf.as_ptr(),
        data_len: buf.len(),
        width,
        height,
        _lifetime: &buf[0],
    }
}

fn image_make_greyscale<'b>(img: &RbImageRgbaRef<'b>) -> RbImageLuma {
    let image: GrayImage = ImageRgbaRef::from(img).convert();

    let data = image.into_raw();
    RbImageLuma {
        data,
        width: img.width,
        height: img.height,
    }
}

fn image_rotate_hue<'bi>(img: &RbImageRgbaRef<'bi>, degrees: i32) -> RbImageRgba {
    let image = ImageRgbaRef::from(img);
    let image = huerotate(&image, degrees);
    let data = image.into_raw();
    RbImageRgba {
        data,
        width: img.width,
        height: img.height,
    }
}

impl<'b> From<&RbImageRgbaRef<'b>> for ImageRgbaRef<'b> {
    fn from(img: &RbImageRgbaRef<'b>) -> Self {
        let buffer = if img.data_len == 0 {
            &[]
        } else {
            assert!(!img.data.is_null());
            // Note(unsafe): an RbImageRgbaRef can only be created using
            // [create_image_ref], which converts a slice into raw parts,
            // and keeps lifetime information intact. We're also doing a
            // null check as well as a check of the data length
            // to be even more sure we're not introducing UB.
            unsafe { std::slice::from_raw_parts(img.data, img.data_len) }
        };

        ImageBuffer::from_raw(img.width, img.height, buffer).unwrap()
    }
}
#[cxx::bridge(namespace = "rb")]
mod bridge {

    #[cxx_name = "ImageRgbaRef"]
    pub struct RbImageRgbaRef<'b> {
        // Sadly, cxx doesn't support
        // slices as fields of shared structs,
        // so we create one ourselves using a
        // raw pointer and its length...
        data: *const u8,
        data_len: usize,
        width: u32,
        height: u32,
        // ... but a slice is incomplete if
        // no lifetime is associated with it.
        // Cxx doesn't currently support std::marker::PhantomData either,
        // so the ~hack~workaround is to add a reference
        // to the first byte in the slice, which *does*
        // convey a lifetime. Trading struct size for
        // more info on our type, so we can be more
        // confident when converting this back to an
        // actual Rust slice.
        _lifetime: &'b u8,
    }

    #[cxx_name = "ImageRgba"]
    pub struct RbImageRgba {
        data: Vec<u8>,
        width: u32,
        height: u32,
    }

    #[cxx_name = "ImageLuma"]
    pub struct RbImageLuma {
        data: Vec<u8>,
        width: u32,
        height: u32,
    }

    extern "Rust" {
        unsafe fn create_image_ref<'b>(
            buf: &'b [u8],
            width: u32,
            height: u32,
        ) -> RbImageRgbaRef<'b>;

        unsafe fn image_make_greyscale<'b>(img: &RbImageRgbaRef<'b>) -> RbImageLuma;

        unsafe fn image_rotate_hue<'b>(img: &RbImageRgbaRef<'b>, degrees: i32) -> RbImageRgba;
    }
}
