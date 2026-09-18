use gpui::{AssetSource, Result, SharedString};
use std::borrow::Cow;

pub const PIN_FILLED_PATH: &str = "icons/pin-filled.svg";
pub const PIN_OUTLINE_PATH: &str = "icons/pin-outline.svg";
pub const FOLDER_PATH: &str = "icons/folder.svg";
pub const RESIZE_GRIP_PATH: &str = "icons/resize-grip.svg";
pub const TRAFFIC_CLOSE_PATH: &str = "icons/traffic-close.svg";
pub const TRAFFIC_MIN_PATH: &str = "icons/traffic-min.svg";
pub const TRAFFIC_ZOOM_PATH: &str = "icons/traffic-zoom.svg";

pub const PIN_FILLED_SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="#1F2328" stroke="#1F2328" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
  <line x1="12" y1="17" x2="12" y2="22"></line>
  <path d="M5 17h14v-1.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6h1a2 2 0 0 0 0-4H8a2 2 0 0 0 0 4h1v4.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24Z"></path>
</svg>"##;

pub const PIN_OUTLINE_SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#9CA3AF" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
  <line x1="12" y1="17" x2="12" y2="22"></line>
  <path d="M5 17h14v-1.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6h1a2 2 0 0 0 0-4H8a2 2 0 0 0 0 4h1v4.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24Z"></path>
</svg>"##;

pub const FOLDER_SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#9CA3AF" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
  <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
</svg>"##;

pub const RESIZE_GRIP_SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="#D1D5DB" stroke-width="1.2" stroke-linecap="round">
  <line x1="10" y1="3" x2="3" y2="10"></line>
  <line x1="10" y1="7" x2="7" y2="10"></line>
</svg>"##;

pub const TRAFFIC_CLOSE_SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="6" height="6" viewBox="0 0 6 6" stroke="#4D0000" stroke-width="1.2" stroke-linecap="round">
  <line x1="1" y1="1" x2="5" y2="5"></line>
  <line x1="5" y1="1" x2="1" y2="5"></line>
</svg>"##;

pub const TRAFFIC_MIN_SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="6" height="6" viewBox="0 0 6 6" stroke="#5C3C00" stroke-width="1.2" stroke-linecap="round">
  <line x1="0.8" y1="3" x2="5.2" y2="3"></line>
</svg>"##;

pub const TRAFFIC_ZOOM_SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="6" height="6" viewBox="0 0 6 6" fill="#003300">
  <polygon points="0.8,0.8 3.2,0.8 0.8,3.2"></polygon>
  <polygon points="5.2,5.2 2.8,5.2 5.2,2.8"></polygon>
</svg>"##;

pub struct EmbeddedAssets;

impl AssetSource for EmbeddedAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        match path {
            PIN_FILLED_PATH => Ok(Some(Cow::Borrowed(PIN_FILLED_SVG))),
            PIN_OUTLINE_PATH => Ok(Some(Cow::Borrowed(PIN_OUTLINE_SVG))),
            FOLDER_PATH => Ok(Some(Cow::Borrowed(FOLDER_SVG))),
            RESIZE_GRIP_PATH => Ok(Some(Cow::Borrowed(RESIZE_GRIP_SVG))),
            TRAFFIC_CLOSE_PATH => Ok(Some(Cow::Borrowed(TRAFFIC_CLOSE_SVG))),
            TRAFFIC_MIN_PATH => Ok(Some(Cow::Borrowed(TRAFFIC_MIN_SVG))),
            TRAFFIC_ZOOM_PATH => Ok(Some(Cow::Borrowed(TRAFFIC_ZOOM_SVG))),
            _ => Ok(None),
        }
    }

    fn list(&self, _path: &str) -> Result<Vec<SharedString>> {
        Ok(vec![
            PIN_FILLED_PATH.into(),
            PIN_OUTLINE_PATH.into(),
            FOLDER_PATH.into(),
            RESIZE_GRIP_PATH.into(),
            TRAFFIC_CLOSE_PATH.into(),
            TRAFFIC_MIN_PATH.into(),
            TRAFFIC_ZOOM_PATH.into(),
        ])
    }
}
