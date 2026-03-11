use wiard::api::Win32::System::Ole::IDropTarget_Impl;
use wiard::api::Win32::System::Ole::*;
use wiard::api::core::implement;

#[implement(IDropTarget)]
struct CustomDropTarget {
    wiard_target: wiard::drag_drop::DropTarget,
}

impl CustomDropTarget {
    fn new(handle: &wiard::WindowHandle) -> Self {
        Self {
            wiard_target: wiard::drag_drop::DropTarget::new(handle),
        }
    }
}

impl IDropTarget_Impl for CustomDropTarget_Impl {
    fn DragEnter(
        &self,
        pdataobj: wiard::api::core::Ref<windows::Win32::System::Com::IDataObject>,
        grfkeystate: wiard::api::Win32::System::SystemServices::MODIFIERKEYS_FLAGS,
        pt: &wiard::api::Win32::Foundation::POINTL,
        pdweffect: *mut DROPEFFECT,
    ) -> wiard::api::core::Result<()> {
        println!("call CustomDropTarget::DragEnter");
        unsafe {
            self.wiard_target.as_raw().DragEnter(
                pdataobj.as_ref().unwrap(),
                grfkeystate,
                *pt,
                pdweffect,
            )?;
            Ok(())
        }
    }

    fn DragOver(
        &self,
        grfkeystate: wiard::api::Win32::System::SystemServices::MODIFIERKEYS_FLAGS,
        pt: &wiard::api::Win32::Foundation::POINTL,
        pdweffect: *mut DROPEFFECT,
    ) -> wiard::api::core::Result<()> {
        println!("call CustomDropTarget::DragOver");
        unsafe {
            self.wiard_target
                .as_raw()
                .DragOver(grfkeystate, *pt, pdweffect)?;
            Ok(())
        }
    }

    fn DragLeave(&self) -> wiard::api::core::Result<()> {
        println!("call CustomDropTarget::DragLeave");
        unsafe {
            self.wiard_target.as_raw().DragLeave()?;
            Ok(())
        }
    }

    fn Drop(
        &self,
        pdataobj: wiard::api::core::Ref<windows::Win32::System::Com::IDataObject>,
        grfkeystate: wiard::api::Win32::System::SystemServices::MODIFIERKEYS_FLAGS,
        pt: &wiard::api::Win32::Foundation::POINTL,
        pdweffect: *mut DROPEFFECT,
    ) -> wiard::api::core::Result<()> {
        println!("call CustomDropTarget::Drop");
        unsafe {
            self.wiard_target.as_raw().Drop(
                pdataobj.as_ref().unwrap(),
                grfkeystate,
                *pt,
                pdweffect,
            )?;
            Ok(())
        }
    }
}

fn main() -> anyhow::Result<()> {
    let mut event_rx = wiard::EventReceiver::new();
    let _window = wiard::Window::builder(&event_rx)
        .title("wiard custom drag drop")
        .drop_target(|window| CustomDropTarget::new(window).into())
        .build()?;
    loop {
        let Some((event, _)) = event_rx.recv() else {
            break;
        };
        match event {
            wiard::Event::DragEnter(ev) => {
                println!("{ev:?}");
            }
            wiard::Event::DragOver(ev) => {
                println!("{ev:?}");
            }
            wiard::Event::DragLeave => {
                println!("DragLeave");
            }
            wiard::Event::Drop(ev) => {
                println!("{ev:?}");
            }
            _ => {}
        }
    }
    Ok(())
}
