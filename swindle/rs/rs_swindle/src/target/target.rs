//! Pure Rust Target representation and lifecycle management.

use core::ffi::c_void;
use super::driver::TargetDriver;

/// Representation of a target instance in pure Rust.
#[repr(C)]
pub struct Target {
    pub driver: &'static str,
    pub core: &'static str,
    pub cpuid: u32,
    pub designer_code: u16,
    pub part_id: u16,
    pub target_options: u32,
    pub regs_size: usize,
    pub attached: bool,
    pub no_hw_breakpoint: bool,
    pub is_cortexm: bool,
    pub flash_mode: bool,
    pub enter_flash_mode_stub: bool,
    pub priv_state: *mut c_void,
    pub priv_free: Option<unsafe fn(priv_: *mut c_void)>,
    pub attach: Option<unsafe fn(t: *mut Target) -> bool>,
    pub detach: Option<unsafe fn(t: *mut Target)>,
    pub mass_erase: Option<unsafe fn(target: *mut Target, context: *mut c_void) -> bool>,
    pub next: *mut Target,
}

pub type TargetAttach = unsafe fn(t: *mut Target) -> bool;
pub type TargetDetach = unsafe fn(t: *mut Target);
pub type PrivFreeHook = unsafe fn(priv_: *mut c_void);
pub type TargetMassErase = unsafe fn(target: *mut Target, context: *mut c_void) -> bool;

unsafe extern "C" {
    pub fn platform_target_clk_output_enable(enable: bool);
    pub fn bmp_get_temporary_buffer(asked: u32) -> *mut c_void;
    fn platform_delay(ms: u32);
}

/// Busy-wait `ms` milliseconds on the probe (platform_delay).
pub fn delay_ms(ms: u32) {
    unsafe { platform_delay(ms) }
}

const TOPT_ARCH_SHIFT: u32 = 28;
const TOPT_ARCH_MASK: u32 = 3;
pub const TOPT_ARCH_ARM: u32 = 1;
pub const TOPT_ARCH_RISCV: u32 = 2;

/// Handle on a live `Target`.
#[derive(Clone, Copy)]
pub struct TargetView {
    target: *mut Target,
}

impl TargetView {
    pub fn new(target: *mut Target) -> Self {
        Self { target }
    }

    pub fn raw(self) -> *mut Target {
        self.target
    }

    pub fn is_null(self) -> bool {
        self.target.is_null()
    }

    pub fn is_cortexm(self) -> bool {
        if self.is_null() {
            false
        } else {
            unsafe { (*self.target).is_cortexm }
        }
    }

    pub fn set_is_cortexm(self, is_cm: bool) {
        if !self.is_null() {
            unsafe { (*self.target).is_cortexm = is_cm; }
        }
    }

    // --- identity -------------------------------------------------------------

    pub fn set_arch_arm(self) {
        if self.is_null() {
            return;
        }
        let options = self.options();
        self.set_options(
            (options & !(TOPT_ARCH_MASK << TOPT_ARCH_SHIFT)) | (TOPT_ARCH_ARM << TOPT_ARCH_SHIFT),
        );
    }

    pub fn set_arch_riscv(self) {
        if self.is_null() {
            return;
        }
        let options = self.options();
        self.set_options(
            (options & !(TOPT_ARCH_MASK << TOPT_ARCH_SHIFT)) | (TOPT_ARCH_RISCV << TOPT_ARCH_SHIFT),
        );
    }

    pub fn designer_code(self) -> u32 {
        if self.is_null() {
            0
        } else {
            unsafe { (*self.target).designer_code as u32 }
        }
    }

    pub fn set_designer_code(self, designer_code: u32) {
        if !self.is_null() {
            unsafe { (*self.target).designer_code = designer_code as u16; }
        }
    }

    pub fn part_id(self) -> u32 {
        if self.is_null() {
            0
        } else {
            unsafe { (*self.target).part_id as u32 }
        }
    }

    pub fn set_part_id(self, part_id: u32) {
        if !self.is_null() {
            unsafe { (*self.target).part_id = part_id as u16; }
        }
    }

    pub fn cpuid(self) -> u32 {
        if self.is_null() {
            0
        } else {
            unsafe { (*self.target).cpuid }
        }
    }

    pub fn set_cpuid(self, cpuid: u32) {
        if !self.is_null() {
            unsafe { (*self.target).cpuid = cpuid; }
        }
    }

    pub fn set_core(self, name: &'static str) {
        if !self.is_null() {
            unsafe { (*self.target).core = name; }
        }
    }

    pub fn core_str(self) -> Option<&'static str> {
        if self.is_null() {
            None
        } else {
            let s = unsafe { (*self.target).core };
            if s.is_empty() { None } else { Some(s) }
        }
    }

    pub fn set_driver(self, name: &'static str) {
        if !self.is_null() {
            unsafe { (*self.target).driver = name; }
        }
    }

    pub fn set_driver_str(self, name: &'static str) {
        self.set_driver(name);
    }

    pub fn driver_str(self) -> Option<&'static str> {
        if self.is_null() {
            None
        } else {
            let s = unsafe { (*self.target).driver };
            if s.is_empty() { None } else { Some(s) }
        }
    }

    pub fn attached(self) -> bool {
        if self.is_null() {
            false
        } else {
            unsafe { (*self.target).attached }
        }
    }

    pub fn set_attached(self, attached: bool) {
        if !self.is_null() {
            unsafe { (*self.target).attached = attached; }
        }
    }

    // --- target options -------------------------------------------------------

    pub fn options(self) -> u32 {
        if self.is_null() {
            0
        } else {
            unsafe { (*self.target).target_options }
        }
    }

    pub fn set_options(self, options: u32) {
        if !self.is_null() {
            unsafe { (*self.target).target_options = options; }
        }
    }

    pub fn or_options(self, bits: u32) {
        if self.is_null() {
            return;
        }
        self.set_options(self.options() | bits);
    }

    pub fn no_hw_breakpoint(self) -> bool {
        if self.is_null() {
            false
        } else {
            unsafe { (*self.target).no_hw_breakpoint }
        }
    }

    pub fn set_no_hw_breakpoint(self, no_hw: bool) {
        if !self.is_null() {
            unsafe { (*self.target).no_hw_breakpoint = no_hw; }
        }
    }

    pub fn has_hw_breakpoint(self) -> bool {
        !self.no_hw_breakpoint()
    }

    // --- private state --------------------------------------------------------

    pub fn priv_ptr(self) -> *mut c_void {
        if self.is_null() {
            core::ptr::null_mut()
        } else {
            unsafe { (*self.target).priv_state }
        }
    }

    pub fn set_priv<T>(self, state: *mut T) {
        if !self.is_null() {
            unsafe { (*self.target).priv_state = state.cast(); }
        }
    }

    pub fn set_priv_free(self, hook: PrivFreeHook) {
        if !self.is_null() {
            unsafe { (*self.target).priv_free = Some(hook); }
        }
    }

    // --- register file --------------------------------------------------------

    pub fn regs_size(self) -> usize {
        if self.is_null() {
            0
        } else {
            unsafe { (*self.target).regs_size }
        }
    }

    pub fn set_regs_size(self, regs_size: usize) {
        if !self.is_null() {
            unsafe { (*self.target).regs_size = regs_size; }
        }
    }

    // --- attach/detach -------------------------------------------------------

    pub fn set_attach(self, hook: TargetAttach) {
        if !self.is_null() {
            unsafe { (*self.target).attach = Some(hook); }
        }
    }

    pub fn set_detach(self, hook: TargetDetach) {
        if !self.is_null() {
            unsafe { (*self.target).detach = Some(hook); }
        }
    }

    pub fn set_mass_erase(self, hook: TargetMassErase) {
        if !self.is_null() {
            unsafe { (*self.target).mass_erase = Some(hook); }
        }
    }

    pub fn set_enter_flash_mode_stub(self) {
        if !self.is_null() {
            unsafe { (*self.target).enter_flash_mode_stub = true; }
        }
    }

    pub fn interrupted(self) -> bool {
        false
    }
}

static CUR_TARGET: crate::sync_cell::SyncCell<*mut Target> =
    crate::sync_cell::SyncCell::new(core::ptr::null_mut());

pub fn get_cur_target() -> *mut Target {
    unsafe { *CUR_TARGET.as_ptr() }
}

pub fn set_cur_target(target: *mut Target) {
    unsafe { *CUR_TARGET.as_ptr() = target };
}

pub fn target_ram_count(target: *mut Target) -> usize {
    if target.is_null() {
        return 0;
    }
    if crate::riscv::get_riscv_context(target).is_some()
        && let Some(driver) = crate::riscv::driver_registry::active_driver()
    {
        return driver.memory_map().ram_regions().len();
    }
    if let Some(driver) = crate::arm::driver_registry::active_driver_for(target) {
        return driver.memory_map().ram_regions().len();
    }
    0
}

pub fn target_ram_get(target: *mut Target, index: usize) -> Option<(u32, u32)> {
    if target.is_null() {
        return None;
    }
    if crate::riscv::get_riscv_context(target).is_some()
        && let Some(driver) = crate::riscv::driver_registry::active_driver()
    {
        return driver
            .memory_map()
            .ram_regions()
            .get(index)
            .map(|r| (r.start, r.length as u32));
    }
    if let Some(driver) = crate::arm::driver_registry::active_driver_for(target) {
        return driver
            .memory_map()
            .ram_regions()
            .get(index)
            .map(|r| (r.start, r.length as u32));
    }
    None
}

pub fn target_flash_count(target: *mut Target) -> usize {
    if target.is_null() {
        return 0;
    }
    if crate::riscv::get_riscv_context(target).is_some()
        && let Some(driver) = crate::riscv::driver_registry::active_driver()
    {
        return driver.memory_map().flash_regions().len();
    }
    if let Some(driver) = crate::arm::driver_registry::active_driver_for(target) {
        return driver.memory_map().flash_regions().len();
    }
    0
}

pub fn target_flash_get(target: *mut Target, index: usize) -> Option<(u32, u32, u32)> {
    if target.is_null() {
        return None;
    }
    if crate::riscv::get_riscv_context(target).is_some()
        && let Some(driver) = crate::riscv::driver_registry::active_driver()
    {
        return driver
            .memory_map()
            .flash_regions()
            .get(index)
            .map(|f| (f.start, f.length as u32, f.blocksize as u32));
    }
    if let Some(driver) = crate::arm::driver_registry::active_driver_for(target) {
        return driver
            .memory_map()
            .flash_regions()
            .get(index)
            .map(|f| (f.start, f.length as u32, f.blocksize as u32));
    }
    None
}

pub fn target_addr_is_flash(target: *mut Target, addr: u32) -> bool {
    if target.is_null() {
        return false;
    }
    let count = target_flash_count(target);
    for idx in 0..count {
        if let Some((start, len, _)) = target_flash_get(target, idx)
            && start <= addr
            && addr < start.wrapping_add(len)
        {
            return true;
        }
    }
    false
}

/// Global target list pointer.
pub static mut target_list: *mut Target = core::ptr::null_mut();

// ---------------------------------------------------------------------------
// Pure Rust Target Lifecycle Management
// ---------------------------------------------------------------------------

pub unsafe fn target_new() -> *mut Target {
    unsafe {
        let size = core::mem::size_of::<Target>();
        let target = crate::session_arena::swindle_session_calloc(1, size) as *mut Target;
        if target.is_null() {
            return core::ptr::null_mut();
        }
        (*target).driver = "";
        (*target).core = "";

        if target_list.is_null() {
            target_list = target;
        } else {
            let mut cur = target_list;
            while !(*cur).next.is_null() {
                cur = (*cur).next;
            }
            (*cur).next = target;
        }

        target
    }
}

pub unsafe fn target_attach_n(n: usize) -> *mut Target {
    unsafe {
        let mut idx = 0;
        let mut cur = target_list;
        while !cur.is_null() {
            idx += 1;
            if idx == n {
                return target_attach(cur);
            }
            cur = (*cur).next;
        }
        core::ptr::null_mut()
    }
}

pub unsafe fn target_attach(target: *mut Target) -> *mut Target {
    if target.is_null() {
        return core::ptr::null_mut();
    }

    unsafe {
        platform_target_clk_output_enable(true);

        let attached = if let Some(mut d) = super::driver_for(target) {
            d.attach().is_ok()
        } else if let Some(attach_fn) = (*target).attach {
            attach_fn(target)
        } else {
            false
        };

        if !attached {
            platform_target_clk_output_enable(false);
            return core::ptr::null_mut();
        }

        (*target).attached = true;
        target
    }
}

pub unsafe fn target_detach(target: *mut Target) {
    if target.is_null() {
        return;
    }
    unsafe {
        if let Some(mut d) = super::driver_for(target) {
            d.detach();
        } else if let Some(detach_fn) = (*target).detach {
            detach_fn(target);
        }
        (*target).attached = false;
        platform_target_clk_output_enable(false);
    }
}

pub unsafe fn target_list_free() {
    unsafe {
        let mut cur = target_list;
        while !cur.is_null() {
            let next = (*cur).next;

            if (*cur).attached {
                if let Some(mut d) = super::driver_for(cur) {
                    d.detach();
                } else if let Some(detach_fn) = (*cur).detach {
                    detach_fn(cur);
                }
            }
            if get_cur_target() == cur {
                set_cur_target(core::ptr::null_mut());
            }

            if !(*cur).priv_state.is_null() {
                if let Some(priv_free) = (*cur).priv_free {
                    priv_free((*cur).priv_state);
                }
            }

            crate::session_arena::swindle_session_free(cur.cast());

            cur = next;
        }
        target_list = core::ptr::null_mut();
        set_cur_target(core::ptr::null_mut());
        crate::session_arena::swindle_session_open();
    }
}

pub unsafe fn target_check_error(target: *mut Target) -> bool {
    if target.is_null() {
        return false;
    }
    if crate::arm::cortexm::is_cortexm(target) {
        crate::arm::cortexm::check_error(target)
    } else {
        false
    }
}
