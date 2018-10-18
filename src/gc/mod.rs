use std::cmp::{Ord, Ordering, PartialOrd};
use std::fmt;

use ctxt::SemContext;
use driver::cmd::{Args, CollectorName};
use gc::copy::CopyCollector;
use gc::space::{Space, SpaceConfig};
use gc::swiper::Swiper;
use gc::tlab::TLAB_OBJECT_SIZE;
use gc::zero::ZeroCollector;
use mem;

pub mod arena;
pub mod chunk;
pub mod copy;
pub mod root;
pub mod space;
pub mod swiper;
pub mod tlab;
pub mod zero;

const LARGE_OBJECT_SIZE: usize = 64 * 1024;

const CHUNK_SIZE: usize = 8 * 1024;
pub const DEFAULT_CODE_SPACE_LIMIT: usize = 128 * 1024;
pub const DEFAULT_PERM_SPACE_LIMIT: usize = 64 * 1024;

pub struct Gc {
    collector: Box<Collector>,

    code_space: Space,
    perm_space: Space,
}

impl Gc {
    pub fn new(args: &Args) -> Gc {
        let code_config = SpaceConfig {
            executable: true,
            chunk: CHUNK_SIZE,
            limit: args.code_size(),
            align: 64,
        };

        let perm_config = SpaceConfig {
            executable: false,
            chunk: CHUNK_SIZE,
            limit: args.perm_size(),
            align: 8,
        };

        let collector_name = args.flag_gc.unwrap_or(CollectorName::Swiper);

        let collector: Box<Collector> = match collector_name {
            CollectorName::Zero => box ZeroCollector::new(args),
            CollectorName::Copy => box CopyCollector::new(args),
            CollectorName::Swiper => box Swiper::new(args),
        };

        Gc {
            collector: collector,

            code_space: Space::new(code_config, "code"),
            perm_space: Space::new(perm_config, "perm"),
        }
    }

    pub fn needs_write_barrier(&self) -> bool {
        self.collector.needs_write_barrier()
    }

    pub fn card_table_offset(&self) -> usize {
        self.collector.card_table_offset()
    }

    pub fn alloc_code(&self, size: usize) -> *mut u8 {
        self.code_space.alloc(size).to_mut_ptr()
    }

    pub fn alloc_perm(&self, size: usize) -> *mut u8 {
        self.perm_space.alloc(size).to_mut_ptr()
    }

    pub fn alloc(&self, ctxt: &SemContext, size: usize, array_ref: bool) -> Address {
        if ctxt.args.flag_gc_stress_minor {
            self.minor_collect(ctxt);
        }

        if ctxt.args.flag_gc_stress {
            self.collect(ctxt);
        }

        if size < TLAB_OBJECT_SIZE && !ctxt.args.flag_disable_tlab {
            self.collector.alloc_tlab(ctxt, size, array_ref)
        } else if size < LARGE_OBJECT_SIZE {
            self.collector.alloc_normal(ctxt, size, array_ref)
        } else {
            self.collector.alloc_large(ctxt, size, array_ref)
        }
    }

    pub fn collect(&self, ctxt: &SemContext) {
        self.collector.collect(ctxt);
    }

    pub fn minor_collect(&self, ctxt: &SemContext) {
        self.collector.minor_collect(ctxt);
    }
}

trait Collector {
    // allocate object of given size
    fn alloc_tlab(&self, ctxt: &SemContext, size: usize, array_ref: bool) -> Address;
    fn alloc_normal(&self, ctxt: &SemContext, size: usize, array_ref: bool) -> Address;
    fn alloc_large(&self, ctxt: &SemContext, size: usize, array_ref: bool) -> Address;

    // collect garbage
    fn collect(&self, ctxt: &SemContext);

    // collect young generation if supported, otherwise
    // collects whole heap
    fn minor_collect(&self, ctxt: &SemContext);

    // decides whether to emit write barriers needed for
    // generational GC to write into card table
    fn needs_write_barrier(&self) -> bool {
        false
    }

    // only need if write barriers needed
    fn card_table_offset(&self) -> usize {
        0
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct Address(usize);

impl Address {
    #[inline(always)]
    pub fn from(val: usize) -> Address {
        Address(val)
    }

    #[inline(always)]
    pub fn region_start(self, size: usize) -> Region {
        Region::new(self, self.offset(size))
    }

    #[inline(always)]
    pub fn offset_from(self, base: Address) -> usize {
        debug_assert!(self >= base);

        self.to_usize() - base.to_usize()
    }

    #[inline(always)]
    pub fn offset(self, offset: usize) -> Address {
        Address(self.0 + offset)
    }

    #[inline(always)]
    pub fn add_ptr(self, ptr: usize) -> Address {
        Address(self.0 + ptr * mem::ptr_width_usize())
    }

    #[inline(always)]
    pub fn to_usize(self) -> usize {
        self.0
    }

    #[inline(always)]
    pub fn from_ptr<T>(ptr: *const T) -> Address {
        Address(ptr as usize)
    }

    #[inline(always)]
    pub fn to_ptr<T>(&self) -> *const T {
        self.0 as *const T
    }

    #[inline(always)]
    pub fn to_mut_ptr<T>(&self) -> *mut T {
        self.0 as *const T as *mut T
    }

    #[inline(always)]
    pub fn null() -> Address {
        Address(0)
    }

    #[inline(always)]
    pub fn is_null(self) -> bool {
        self.0 == 0
    }

    #[inline(always)]
    pub fn is_non_null(self) -> bool {
        self.0 != 0
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "0x{:x}", self.to_usize())
    }
}

impl PartialOrd for Address {
    fn partial_cmp(&self, other: &Address) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Address {
    fn cmp(&self, other: &Address) -> Ordering {
        self.to_usize().cmp(&other.to_usize())
    }
}

impl From<usize> for Address {
    fn from(val: usize) -> Address {
        Address(val)
    }
}

#[derive(Clone)]
pub struct Region {
    pub start: Address,
    pub end: Address,
}

impl Region {
    pub fn new(start: Address, end: Address) -> Region {
        Region {
            start: start,
            end: end,
        }
    }

    #[inline(always)]
    pub fn contains(&self, addr: Address) -> bool {
        self.start <= addr && addr < self.end
    }

    #[inline(always)]
    pub fn valid_top(&self, addr: Address) -> bool {
        self.start <= addr && addr <= self.end
    }

    #[inline(always)]
    pub fn size(&self) -> usize {
        self.end.to_usize() - self.start.to_usize()
    }
}

impl fmt::Display for Region {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}-{}", self.start, self.end)
    }
}

struct FormattedSize {
    size: usize,
}

impl fmt::Display for FormattedSize {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ksize = (self.size as f64) / 1024f64;

        if ksize < 1f64 {
            return write!(f, "{}B", self.size);
        }

        let msize = ksize / 1024f64;

        if msize < 1f64 {
            return write!(f, "{:.1}K", ksize);
        }

        let gsize = msize / 1024f64;

        if gsize < 1f64 {
            write!(f, "{:.1}M", msize)
        } else {
            write!(f, "{:.1}G", gsize)
        }
    }
}

fn formatted_size(size: usize) -> FormattedSize {
    FormattedSize { size }
}