//! Kernel-hosted window stack (z-order) for UEFI Fluent bring-up.
//! Future: replace with PE processes posting to user32 (`milestones::PHASE_WIN32K_GRAPHICS`).

pub const MAX_HOSTED_WINDOWS: usize = 8;

/// Openable shell applications; `title_idx()` indexes `resources::window_title_bgra` (except Terminal).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AppId {
    #[default]
    Files = 0,
    TaskMgr = 1,
    Settings = 2,
    ControlPanel = 3,
    Run = 4,
    Notepad = 5,
    Calculator = 6,
    About = 7,
    Properties = 8,
}

impl AppId {
    #[must_use]
    pub const fn title_idx(self) -> usize {
        self as usize
    }

    #[must_use]
    pub const fn to_u8(self) -> u8 {
        self as u8
    }

    #[must_use]
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Files),
            1 => Some(Self::TaskMgr),
            2 => Some(Self::Settings),
            3 => Some(Self::ControlPanel),
            4 => Some(Self::Run),
            5 => Some(Self::Notepad),
            6 => Some(Self::Calculator),
            7 => Some(Self::About),
            8 => Some(Self::Properties),
            _ => None,
        }
    }

    /// `Start` menu row index → app to open (`None` = stub / power flow handled in session).
    #[must_use]
    pub fn from_menu_row(row: usize) -> Option<Self> {
        match row {
            1 => Some(Self::Files),
            2 => Some(Self::TaskMgr),
            3 => Some(Self::Settings),
            4 => Some(Self::ControlPanel),
            5 => Some(Self::Run),
            6 => Some(Self::Notepad),
            7 => Some(Self::Calculator),
            8 => Some(Self::Files),
            9 => Some(Self::About),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct WindowStack {
    /// Front / focus at index `len - 1`.
    pub ids: [AppId; MAX_HOSTED_WINDOWS],
    pub len: u8,
}

impl Default for WindowStack {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowStack {
    pub const fn new() -> Self {
        Self {
            ids: [AppId::Files; MAX_HOSTED_WINDOWS],
            len: 0,
        }
    }

    /// Bring `id` to front; evict oldest if full.
    pub fn push_front(&mut self, id: AppId) {
        if self.len == 0 {
            self.ids[0] = id;
            self.len = 1;
            return;
        }
        let n = self.len as usize;
        let mut found = None;
        for i in 0..n {
            if self.ids[i] == id {
                found = Some(i);
                break;
            }
        }
        if let Some(i) = found {
            let v = self.ids[i];
            let mut j = i;
            while j + 1 < n {
                self.ids[j] = self.ids[j + 1];
                j += 1;
            }
            self.ids[n - 1] = v;
            return;
        }
        if n < MAX_HOSTED_WINDOWS {
            self.ids[n] = id;
            self.len += 1;
            return;
        }
        for j in 0..MAX_HOSTED_WINDOWS - 1 {
            self.ids[j] = self.ids[j + 1];
        }
        self.ids[MAX_HOSTED_WINDOWS - 1] = id;
    }

    pub fn pop_top(&mut self) -> Option<AppId> {
        if self.len == 0 {
            return None;
        }
        let n = (self.len - 1) as usize;
        let v = self.ids[n];
        self.len -= 1;
        Some(v)
    }

    pub fn remove(&mut self, id: AppId) {
        let n = self.len as usize;
        let mut w = 0usize;
        for r in 0..n {
            if self.ids[r] != id {
                self.ids[w] = self.ids[r];
                w += 1;
            }
        }
        self.len = w as u8;
    }

    #[must_use]
    pub fn top(&self) -> Option<AppId> {
        if self.len == 0 {
            None
        } else {
            Some(self.ids[(self.len - 1) as usize])
        }
    }

    #[must_use]
    pub fn contains(&self, id: AppId) -> bool {
        for i in 0..self.len as usize {
            if self.ids[i] == id {
                return true;
            }
        }
        false
    }

    /// Minimize-all / show-desktop style: drop every hosted window.
    pub fn clear(&mut self) {
        self.len = 0;
    }
}
