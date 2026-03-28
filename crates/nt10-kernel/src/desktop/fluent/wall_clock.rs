//! Wall clock for the taskbar notification area.
//!
//! Windows documents the taskbar clock and notification area in:
//! - `references/win32/desktop-src/shell/taskbar.md` (notification area, optional clock)
//! - `references/win32/desktop-src/shell/notification-area.md`
//! - `references/win32/desktop-src/uxguide/winenv-notification.md`
//!
//! On PC firmware we read CMOS RTC (ports 0x70/0x71). Other targets fall back to a poll-based uptime
//! display (no wall time).

/// Decoded RTC fields when CMOS read succeeds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RtcSnapshot {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

#[cfg(target_arch = "x86_64")]
mod cmos {
    use super::RtcSnapshot;

    #[inline]
    unsafe fn outb(port: u16, val: u8) {
        core::arch::asm!("out dx, al", in("dx") port, in("al") val, options(nostack, preserves_flags));
    }

    #[inline]
    unsafe fn inb(port: u16) -> u8 {
        let v: u8;
        core::arch::asm!("in al, dx", out("al") v, in("dx") port, options(nostack, preserves_flags));
        v
    }

    #[inline]
    unsafe fn cmos_read(reg: u8) -> u8 {
        outb(0x70, reg);
        inb(0x71)
    }

    #[inline]
    fn from_bcd(v: u8) -> u8 {
        ((v >> 4) * 10) + (v & 0x0F)
    }

    /// Best-effort CMOS read; returns `None` on obviously invalid values.
    pub fn read_rtc() -> Option<RtcSnapshot> {
        unsafe {
            let st_b = cmos_read(0x0B);
            let hour_24 = (st_b & 2) != 0;

            let sec = from_bcd(cmos_read(0x00) & 0x7F);
            let min = from_bcd(cmos_read(0x02) & 0x7F);
            let hour_raw = cmos_read(0x04);
            let hour = if hour_24 {
                from_bcd(hour_raw & 0x3F)
            } else {
                let pm = hour_raw & 0x80 != 0;
                let h12 = from_bcd(hour_raw & 0x7F & !0x80);
                let h12 = h12.min(12);
                let h24 = if h12 == 12 {
                    if pm { 12 } else { 0 }
                } else if pm {
                    h12.saturating_add(12).min(23)
                } else {
                    h12
                };
                h24
            };
            let day = from_bcd(cmos_read(0x07) & 0x3F);
            let month = from_bcd(cmos_read(0x08) & 0x1F);
            let year2 = from_bcd(cmos_read(0x09) & 0xFF);
            let year = 2000u16 + year2 as u16;

            if sec >= 60 || min >= 60 || hour >= 24 || month == 0 || month > 12 || day == 0 || day > 31 {
                return None;
            }
            Some(RtcSnapshot {
                hour,
                minute: min,
                second: sec,
                year,
                month,
                day,
            })
        }
    }
}

#[cfg(not(target_arch = "x86_64"))]
mod cmos {
    use super::RtcSnapshot;

    pub fn read_rtc() -> Option<RtcSnapshot> {
        None
    }
}

/// Read the real-time clock when available (x86 CMOS).
#[must_use]
pub fn try_read_rtc() -> Option<RtcSnapshot> {
    cmos::read_rtc()
}

/// Format `HH:MM:SS` and `YYYY/MM/DD` into ASCII buffers. Returns `(time_len, date_len)`.
pub fn format_clock_lines(
    rtc: Option<RtcSnapshot>,
    uptime_secs: u32,
    time_out: &mut [u8; 16],
    date_out: &mut [u8; 20],
) -> (usize, usize) {
    let (h, m, s, y, mo, d) = if let Some(r) = rtc {
        (
            r.hour as u32,
            r.minute as u32,
            r.second as u32,
            r.year,
            r.month as u32,
            r.day as u32,
        )
    } else {
        let t = uptime_secs % 86400;
        (
            t / 3600,
            (t % 3600) / 60,
            t % 60,
            2020u16 + ((uptime_secs / 86400) % 4000) as u16,
            1u32,
            1u32,
        )
    };

    fn push2(buf: &mut [u8], pos: &mut usize, v: u32) {
        buf[*pos] = b'0' + (v / 10) as u8;
        buf[*pos + 1] = b'0' + (v % 10) as u8;
        *pos += 2;
    }

    let mut i = 0usize;
    push2(time_out, &mut i, h);
    time_out[i] = b':';
    i += 1;
    push2(time_out, &mut i, m);
    time_out[i] = b':';
    i += 1;
    push2(time_out, &mut i, s);
    let time_len = i;

    let mut j = 0usize;
    let y4 = y as u32;
    date_out[j] = b'0' + (y4 / 1000) as u8;
    date_out[j + 1] = b'0' + ((y4 / 100) % 10) as u8;
    date_out[j + 2] = b'0' + ((y4 / 10) % 10) as u8;
    date_out[j + 3] = b'0' + (y4 % 10) as u8;
    j += 4;
    date_out[j] = b'/';
    j += 1;
    push2(date_out, &mut j, mo);
    date_out[j] = b'/';
    j += 1;
    push2(date_out, &mut j, d);
    let date_len = j;

    (time_len, date_len)
}
