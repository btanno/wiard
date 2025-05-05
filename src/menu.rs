use crate::*;
use std::sync::Arc;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::PWSTR;

#[derive(Debug)]
struct RawHandle {
    raw: HMENU,
}

impl RawHandle {
    fn new(handle: HMENU) -> Self {
        Self { raw: handle }
    }
}

impl Drop for RawHandle {
    fn drop(&mut self) {
        unsafe {
            if IsMenu(self.raw).as_bool() {
                let _ = DestroyMenu(self.raw);
            }
        }
    }
}

unsafe impl Send for RawHandle {}
unsafe impl Sync for RawHandle {}

trait Item {
    fn text(&self) -> &str;
    fn sub_menu(&self) -> Option<Menu>;
}

/// Builds a MenuBarItem.
#[derive(Debug, Default)]
pub struct MenuBarItemBuilder {
    item: MenuBarItem,
}

impl MenuBarItemBuilder {
    #[inline]
    pub fn new() -> Self {
        Self {
            item: MenuBarItem::default(),
        }
    }

    #[inline]
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.item.text = text.into();
        self
    }

    #[inline]
    pub fn sub_menu(mut self, menu: &Menu) -> Self {
        self.item.sub_menu = Some(menu.clone());
        self
    }

    #[inline]
    pub fn right_justify(mut self, flag: bool) -> Self {
        self.item.right_justify = flag;
        self
    }
}

impl From<MenuBarItemBuilder> for MenuBarItem {
    #[inline]
    fn from(value: MenuBarItemBuilder) -> Self {
        value.item
    }
}

/// Represents a item for MenuBar.
#[derive(Debug, Default)]
pub struct MenuBarItem {
    pub text: String,
    pub sub_menu: Option<Menu>,
    pub right_justify: bool,
}

impl MenuBarItem {
    #[inline]
    pub fn builder() -> MenuBarItemBuilder {
        MenuBarItemBuilder::new()
    }
}

impl Item for MenuBarItem {
    fn text(&self) -> &str {
        &self.text
    }

    fn sub_menu(&self) -> Option<Menu> {
        self.sub_menu.clone()
    }
}

#[derive(Debug, Default)]
pub struct Text {
    pub text: String,
    pub sub_menu: Option<Menu>,
}

/// Builds a MenuItem.
#[derive(Debug, Default)]
pub struct MenuItemBuilder {
    item: Text,
}

impl MenuItemBuilder {
    #[inline]
    pub fn new() -> Self {
        Self {
            item: Text::default(),
        }
    }

    #[inline]
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.item.text = text.into();
        self
    }

    #[inline]
    pub fn sub_menu(mut self, menu: &Menu) -> Self {
        self.item.sub_menu = Some(menu.clone());
        self
    }
}

impl From<MenuItemBuilder> for MenuItem {
    #[inline]
    fn from(value: MenuItemBuilder) -> Self {
        MenuItem::Text(value.item)
    }
}

/// Represents a item for a Menu.
#[derive(Debug)]
pub enum MenuItem {
    Text(Text),
    Separator,
}

impl MenuItem {
    #[inline]
    pub fn builder() -> MenuItemBuilder {
        MenuItemBuilder::new()
    }

    #[inline]
    pub fn separator() -> Self {
        Self::Separator
    }
}

impl Item for MenuItem {
    fn text(&self) -> &str {
        let Self::Text(item) = self else {
            unreachable!()
        };
        &item.text
    }

    fn sub_menu(&self) -> Option<Menu> {
        let Self::Text(item) = self else {
            unreachable!()
        };
        item.sub_menu.clone()
    }
}

/// Represents a menu handle.
/// MenuHandle can compare with a MenuBar and a Menu.
#[derive(Clone, Copy, Debug)]
pub struct MenuHandle {
    handle: HMENU,
}

impl MenuHandle {
    pub(crate) fn from_raw(handle: HMENU) -> Self {
        Self { handle }
    }
}

unsafe impl Send for MenuHandle {}
unsafe impl Sync for MenuHandle {}

#[derive(Debug)]
struct Object {
    handle: RawHandle,
}

impl Object {
    fn new(handle: HMENU) -> Result<Self> {
        unsafe {
            let mut info = MENUINFO {
                cbSize: std::mem::size_of::<MENUINFO>() as u32,
                ..Default::default()
            };
            GetMenuInfo(handle, &mut info)?;
            SetMenuInfo(
                handle,
                &MENUINFO {
                    cbSize: std::mem::size_of::<MENUINFO>() as u32,
                    fMask: MIM_STYLE | MIM_APPLYTOSUBMENUS | info.fMask,
                    dwStyle: MNS_NOTIFYBYPOS | info.dwStyle,
                    ..Default::default()
                },
            )?;
            Ok(Self {
                handle: RawHandle::new(handle),
            })
        }
    }

    fn len(&self) -> usize {
        unsafe { GetMenuItemCount(Some(self.handle.raw)) as usize }
    }

    fn insert(&self, index: usize, item: impl Item) -> Result<usize> {
        unsafe {
            let mut text = item
                .text()
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect::<Vec<_>>();
            let item = MENUITEMINFOW {
                cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
                fMask: MIIM_STRING
                    | item
                        .sub_menu()
                        .as_ref()
                        .map_or(MENU_ITEM_MASK(0), |_| MIIM_SUBMENU),
                dwTypeData: PWSTR::from_raw(text.as_mut_ptr()),
                cch: text.len() as u32,
                hSubMenu: item
                    .sub_menu()
                    .as_ref()
                    .map_or(HMENU::default(), |sm| sm.object.as_hmenu()),
                ..Default::default()
            };
            InsertMenuItemW(self.handle.raw, index as u32, true, &item)?;
        }
        Ok(index)
    }

    fn remove(&self, index: usize) -> Result<()> {
        unsafe {
            RemoveMenu(self.handle.raw, index as u32, MF_BYPOSITION)?;
        }
        Ok(())
    }

    fn as_hmenu(&self) -> HMENU {
        self.handle.raw
    }
}

/// Represents a horizontal menu.
#[derive(Clone, Debug)]
pub struct MenuBar {
    object: Arc<Object>,
}

impl MenuBar {
    #[inline]
    pub fn new() -> Result<Self> {
        unsafe {
            Ok(Self {
                object: Arc::new(Object::new(CreateMenu()?)?),
            })
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.object.len() == 0
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.object.len()
    }

    #[inline]
    pub fn push(&self, item: impl Into<MenuBarItem>) -> Result<usize> {
        self.insert(self.len(), item)
    }

    #[inline]
    pub fn insert(&self, index: usize, item: impl Into<MenuBarItem>) -> Result<usize> {
        self.object.insert(index, item.into())
    }

    #[inline]
    pub fn remove(&self, index: usize) -> Result<()> {
        self.object.remove(index)
    }

    pub(crate) fn as_hmenu(&self) -> HMENU {
        self.object.as_hmenu()
    }
}

impl PartialEq<MenuHandle> for MenuBar {
    #[inline]
    fn eq(&self, other: &MenuHandle) -> bool {
        self.object.as_hmenu() == other.handle
    }
}

impl PartialEq<MenuBar> for MenuHandle {
    #[inline]
    fn eq(&self, other: &MenuBar) -> bool {
        other == self
    }
}

/// Represents a vertical menu.
#[derive(Clone, Debug)]
pub struct Menu {
    object: Arc<Object>,
}

impl Menu {
    #[inline]
    pub fn new() -> Result<Self> {
        unsafe {
            Ok(Self {
                object: Arc::new(Object::new(CreatePopupMenu()?)?),
            })
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.object.len() == 0
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.object.len()
    }

    #[inline]
    pub fn push(&self, item: impl Into<MenuItem>) -> Result<usize> {
        self.insert(self.len(), item)
    }

    #[inline]
    pub fn insert(&self, index: usize, item: impl Into<MenuItem>) -> Result<usize> {
        let item = item.into();
        match item {
            MenuItem::Separator => unsafe {
                let info = MENUITEMINFOW {
                    cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
                    fMask: MIIM_FTYPE,
                    fType: MFT_SEPARATOR,
                    ..Default::default()
                };
                InsertMenuItemW(self.object.as_hmenu(), index as u32, true, &info)?;
                Ok(index)
            },
            _ => self.object.insert(index, item),
        }
    }

    #[inline]
    pub fn remove(&self, index: usize) -> Result<()> {
        self.object.remove(index)
    }

    #[inline]
    pub fn track(&self, window: &impl IsWindow, position: ScreenPosition<i32>) -> Result<()> {
        let window = window.window_handle();
        let menu = self.object.clone();
        UiThread::send_task(move || unsafe {
            let _ = TrackPopupMenuEx(
                menu.as_hmenu(),
                TPM_LEFTALIGN.0 | TPM_TOPALIGN.0,
                position.x,
                position.y,
                window.as_hwnd(),
                None,
            );
        });
        Ok(())
    }
}

impl PartialEq<MenuHandle> for Menu {
    #[inline]
    fn eq(&self, other: &MenuHandle) -> bool {
        self.object.as_hmenu() == other.handle
    }
}

impl PartialEq<Menu> for MenuHandle {
    #[inline]
    fn eq(&self, other: &Menu) -> bool {
        other == self
    }
}
