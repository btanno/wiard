use crate::*;
use bitflags::bitflags;
use std::path::PathBuf;
use tokio::sync::oneshot;
use windows::Win32::{
    Foundation::*,
    System::{Com::*, Memory::*, Ole::*, SystemServices::*},
    UI::Shell::{DragQueryFileW, HDROP},
};
use windows::core::{Ref, implement};

#[derive(Debug)]
#[non_exhaustive]
pub enum Data {
    Files(Vec<PathBuf>),
}

impl Data {
    fn new(data: &IDataObject) -> windows::core::Result<Option<Self>> {
        unsafe {
            let format = FORMATETC {
                cfFormat: CF_HDROP.0,
                tymed: TYMED_HGLOBAL.0 as u32,
                ..Default::default()
            };
            if data.QueryGetData(&format).is_ok() {
                let mut d = data.GetData(&format)?;
                let hdrop = HDROP(GlobalLock(d.u.hGlobal));
                let len = DragQueryFileW(hdrop, u32::MAX, None);
                let mut files = Vec::with_capacity(len as usize);
                for i in 0..len {
                    let len = DragQueryFileW(hdrop, i, None);
                    let mut buf = vec![0u16; len as usize + 1];
                    DragQueryFileW(hdrop, i, Some(&mut buf));
                    buf.pop();
                    files.push(String::from_utf16_lossy(&buf).into());
                }
                GlobalUnlock(HGLOBAL(hdrop.0))?;
                ReleaseStgMedium(&mut d);
                Ok(Some(Self::Files(files)))
            } else {
                Ok(None)
            }
        }
    }
}

bitflags! {
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
    pub struct Effect: u32 {
        const NONE = DROPEFFECT_NONE.0;
        const COPY = DROPEFFECT_COPY.0;
        const MOVE = DROPEFFECT_MOVE.0;
        const LINK = DROPEFFECT_LINK.0;
        const SCROLL = DROPEFFECT_SCROLL.0;
    }
}

impl Effect {
    pub const fn as_raw(&self) -> DROPEFFECT {
        DROPEFFECT(self.bits())
    }
}

impl From<Effect> for DROPEFFECT {
    #[inline]
    fn from(value: Effect) -> Self {
        value.as_raw()
    }
}

impl From<DROPEFFECT> for Effect {
    #[inline]
    fn from(value: DROPEFFECT) -> Self {
        Self::from_bits_retain(value.0)
    }
}

#[derive(Clone, Debug)]
#[implement(IDropTarget)]
pub struct DropTarget {
    handle: WindowHandle,
}

impl DropTarget {
    #[inline]
    pub fn new(handle: &WindowHandle) -> Self {
        Self {
            handle: handle.clone(),
        }
    }

    #[inline]
    pub fn as_raw(&self) -> IDropTarget {
        self.clone().into()
    }
}

impl IDropTarget_Impl for DropTarget_Impl {
    fn DragEnter(
        &self,
        pdataobj: Ref<IDataObject>,
        grfkeystate: MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        pdweffect: *mut DROPEFFECT,
    ) -> windows_core::Result<()> {
        unsafe {
            let Some(data) = pdataobj.as_ref() else {
                *pdweffect = DROPEFFECT_NONE;
                return Ok(());
            };
            let Some(data) = Data::new(data)? else {
                *pdweffect = DROPEFFECT_NONE;
                return Ok(());
            };
            let (tx, rx) = oneshot::channel();
            let ev = event::DragEnter {
                data,
                position: ScreenPosition::new(pt.x, pt.y),
                modifier_keys: grfkeystate.into(),
                effect: (*pdweffect).into(),
                tx: Some(tx),
            };
            Context::send_event(self.handle, Event::DragEnter(ev));
            let effect = rx.blocking_recv().unwrap();
            *pdweffect = effect.into();
            Ok(())
        }
    }

    fn DragOver(
        &self,
        grfkeystate: MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        pdweffect: *mut DROPEFFECT,
    ) -> windows_core::Result<()> {
        unsafe {
            let (tx, rx) = oneshot::channel();
            let ev = event::DragOver {
                position: ScreenPosition::new(pt.x, pt.y),
                modifier_keys: grfkeystate.into(),
                effect: (*pdweffect).into(),
                tx: Some(tx),
            };
            Context::send_event(self.handle, Event::DragOver(ev));
            let effect = rx.blocking_recv().unwrap();
            *pdweffect = effect.into();
            Ok(())
        }
    }

    fn DragLeave(&self) -> windows_core::Result<()> {
        Context::send_event(self.handle, Event::DragLeave);
        Ok(())
    }

    fn Drop(
        &self,
        pdataobj: Ref<IDataObject>,
        grfkeystate: MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        pdweffect: *mut DROPEFFECT,
    ) -> windows_core::Result<()> {
        unsafe {
            let Some(data) = pdataobj.as_ref() else {
                *pdweffect = DROPEFFECT_NONE;
                return Ok(());
            };
            let Some(data) = Data::new(data)? else {
                *pdweffect = DROPEFFECT_NONE;
                return Ok(());
            };
            let (tx, rx) = oneshot::channel();
            let ev = event::Drop {
                data,
                position: ScreenPosition::new(pt.x, pt.y),
                modifier_keys: grfkeystate.into(),
                effect: (*pdweffect).into(),
                tx: Some(tx),
            };
            Context::send_event(self.handle, Event::Drop(ev));
            let effect = rx.blocking_recv().unwrap();
            *pdweffect = effect.into();
            Ok(())
        }
    }
}
