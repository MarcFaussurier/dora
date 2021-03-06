use parking_lot::{Mutex, MutexGuard};
use std::cmp::{max, min};
use std::mem::replace;

use gc::swiper::card::CardTable;
use gc::swiper::controller::SharedHeapConfig;
use gc::swiper::crossing::CrossingMap;
use gc::swiper::{CARD_REFS, CARD_SIZE, CARD_SIZE_BITS};
use gc::{arena, Address, Region, GEN_SIZE};
use mem;
use object::offset_of_array_data;

pub struct OldGen {
    total: Region,
    protected: Mutex<OldGenProtected>,

    crossing_map: CrossingMap,
    card_table: CardTable,
    config: SharedHeapConfig,
}

impl OldGen {
    pub fn new(
        start: Address,
        end: Address,
        crossing_map: CrossingMap,
        card_table: CardTable,
        config: SharedHeapConfig,
    ) -> OldGen {
        let total = Region::new(start, end);

        let old = OldGen {
            total: total.clone(),
            protected: Mutex::new(OldGenProtected::new(total)),

            crossing_map: crossing_map,
            card_table: card_table,
            config: config,
        };

        old
    }

    pub fn total(&self) -> Region {
        self.total.clone()
    }

    pub fn total_start(&self) -> Address {
        self.total.start
    }

    pub fn committed_size(&self) -> usize {
        let protected = self.protected.lock();

        protected.size
    }

    pub fn active_size(&self) -> usize {
        let protected = self.protected.lock();
        protected.active_size()
    }

    pub fn alloc(&self, size: usize) -> Address {
        let mut protected = self.protected.lock();
        protected.alloc(&self.config, size)
    }

    pub fn contains_slow(&self, addr: Address) -> bool {
        let protected = self.protected.lock();
        protected.contains_slow(addr)
    }

    pub fn update_crossing(&self, old: Address, new: Address, array_ref: bool) {
        debug_assert!(self.total.valid_top(old) && self.total.valid_top(new));

        if (old.to_usize() >> CARD_SIZE_BITS) == (new.to_usize() >> CARD_SIZE_BITS) {
            // object does not span multiple cards

        } else if array_ref {
            let new_card_idx = self.card_table.card_idx(new.into());
            let new_card_start = self.card_table.to_address(new_card_idx);

            let old_card_idx = self.card_table.card_idx(old);
            let old_card_end = self.card_table.to_address(old_card_idx).offset(CARD_SIZE);

            let mut loop_card_start = old_card_idx.to_usize() + 1;

            // If you allocate an object array just before the card end,
            // it could happen that the card starts with part of the header
            // or the length-field.
            if old.offset(offset_of_array_data() as usize) > old_card_end {
                let diff = old_card_end.offset_from(old) / mem::ptr_width_usize();
                self.crossing_map
                    .set_array_start(loop_card_start.into(), diff);

                loop_card_start += 1;
            }

            // all cards between ]old_card; new_card[ are full with references
            for c in loop_card_start..new_card_idx.to_usize() {
                self.crossing_map
                    .set_references_at_start(c.into(), CARD_REFS);
            }

            // new_card starts with x references, then next object
            if new_card_idx.to_usize() >= loop_card_start && new < self.total.end {
                if new == new_card_start {
                    self.crossing_map.set_first_object(new_card_idx, 0);
                } else {
                    let refs_dist = new.offset_from(new_card_start) / mem::ptr_width_usize();
                    self.crossing_map
                        .set_references_at_start(new_card_idx, refs_dist);
                }
            }
        } else {
            let new_card_idx = self.card_table.card_idx(new.into());
            let new_card_start = self.card_table.to_address(new_card_idx);

            let old_card_idx = self.card_table.card_idx(old.into());

            // all cards between ]old_card; new_card[ are set to NoRefs
            for c in old_card_idx.to_usize() + 1..new_card_idx.to_usize() {
                self.crossing_map.set_no_references(c.into());
            }

            // new_card stores x words of object, then next object
            if new < self.total.end {
                self.crossing_map.set_first_object(
                    new_card_idx,
                    new.offset_from(new_card_start) / mem::ptr_width_usize(),
                );
            }
        }
    }

    pub fn protected(&self) -> MutexGuard<OldGenProtected> {
        self.protected.lock()
    }

    pub fn dump_regions(&self) {
        let protected = self.protected.lock();
        protected.dump_regions();
    }
}

pub struct OldGenProtected {
    pub total: Region,
    pub size: usize,
    pub regions: Vec<OldGenRegion>,
    pub alloc_region: usize,
}

impl OldGenProtected {
    fn new(total: Region) -> OldGenProtected {
        OldGenProtected {
            total: total.clone(),
            size: 0,
            regions: vec![OldGenRegion::single(total)],
            alloc_region: 0,
        }
    }

    pub fn contains_slow(&self, addr: Address) -> bool {
        for old_region in &self.regions {
            if old_region.total.start <= addr && addr < old_region.top {
                return true;
            }
        }

        false
    }

    pub fn with_single_region(&mut self, top: Address) -> usize {
        let limit = top.align_gen();
        assert!(self.total.valid_top(limit));

        let mut last_mapped = self.total.start;
        let mut committed = 0;

        for region in &self.regions {
            let mapping_end = min(region.mapping_start(), limit);

            if mapping_end > last_mapped {
                committed += mapping_end.offset_from(last_mapped);
            }

            committed += region.mapping_end().offset_from(region.mapping_start());
            last_mapped = region.mapping_end();
        }

        if limit > last_mapped {
            committed += limit.offset_from(last_mapped);
        }

        committed
    }

    pub fn commit_single_region(&mut self, top: Address) {
        let limit = top.align_gen();
        assert!(self.total.valid_top(limit));

        let mut last_mapped = self.total.start;

        for region in &self.regions {
            let mapping_end = min(region.mapping_start(), limit);

            if mapping_end > last_mapped {
                let size = mapping_end.offset_from(last_mapped);
                arena::commit(last_mapped, size, false);
            }

            if mapping_end == limit {
                return;
            }

            last_mapped = region.mapping_end();
        }

        if limit > last_mapped {
            let size = limit.offset_from(last_mapped);
            arena::commit(last_mapped, size, false);
        }
    }

    pub fn with_regions(&mut self, new_regions: &[Region]) -> usize {
        let mut idx = 0;
        let mut committed = 0;
        let mut start = self.total.start;

        for new in new_regions {
            start = max(new.start, start);
            let end = new.end;

            while idx < self.regions.len() && start < end {
                let old = &self.regions[idx];

                // old region fully after new region
                if old.mapping_start() >= end {
                    break;

                // old region fully before new region
                } else if old.mapping_end() <= start {
                    committed += old.mapping_size();
                    idx += 1;
                    continue;

                // we know now that old and new regions overlap
                } else {
                    // new region starts before old region
                    // memory needs to be committed
                    if start < old.mapping_start() {
                        committed += old.mapping_start().offset_from(start);
                    }

                    committed += old.mapping_size();
                    start = old.mapping_end();
                    idx += 1;
                }
            }

            if start < end {
                committed += end.offset_from(start);
            }
        }

        while idx < self.regions.len() {
            let old = &self.regions[idx];
            committed += old.mapping_size();
            idx += 1;
        }

        committed
    }

    pub fn commit_regions(&mut self, new_regions: &[Region]) {
        let mut idx = 0;
        let mut start = self.total.start;

        for new in new_regions {
            start = max(new.start, start);
            let end = new.end;

            while idx < self.regions.len() && start < end {
                let old = &self.regions[idx];

                // new region fully before new region
                if old.mapping_start() >= end {
                    break;

                // old region fully before new region
                } else if old.mapping_end() <= start {
                    idx += 1;
                    continue;

                // we know now that old and new regions overlap
                } else {
                    // new region starts before old region
                    // memory needs to be committed
                    if start < old.mapping_start() {
                        let size = old.mapping_start().offset_from(start);
                        arena::commit(start, size, false);
                    }

                    start = old.mapping_end();
                    idx += 1;
                }
            }

            if start < end {
                let size = end.offset_from(start);
                arena::commit(start, size, false);
            }
        }
    }

    pub fn update_single_region(&mut self, top: Address) {
        let limit = top.align_gen();
        assert!(self.total.valid_top(limit));

        let region = OldGenRegion {
            total: self.total.clone(),
            total_mapping: self.total.clone(),
            top: top,
            mapping_top: limit,
        };

        let regions = replace(&mut self.regions, vec![region]);

        for region in regions {
            if limit >= region.mapping_end() {
                continue;
            }

            let start = max(region.mapping_start(), limit);
            let size = region.mapping_end().offset_from(start);

            if size > 0 {
                arena::discard(start, size);
            }
        }

        self.size = limit.offset_from(self.total.start);
        self.alloc_region = 0;
    }

    pub fn update_regions(&mut self, new_regions: Vec<OldGenRegion>) {
        let mut idx = 0;
        let mut start = self.total.start;

        for old in &self.regions {
            start = max(old.mapping_start(), start);
            let end = old.mapping_end();

            while idx < new_regions.len() && start < end {
                let new = &new_regions[idx];

                // old region fully before new region
                if new.mapping_start() >= end {
                    break;

                // new region fully before old region
                } else if new.mapping_end() <= start {
                    idx += 1;
                    continue;

                // we know now that old and new regions overlap
                } else {
                    // old region starts before new region
                    // memory needs to be forgotten
                    if start < new.mapping_start() {
                        let size = new.mapping_start().offset_from(start);
                        arena::discard(start, size);
                    }

                    start = new.mapping_end();
                    idx += 1;
                }
            }

            if start < end {
                let size = end.offset_from(start);
                arena::discard(start, size);
            }
        }

        std::mem::replace(&mut self.regions, new_regions);

        let mut size = 0;

        for region in &self.regions {
            size += region.committed_region().size();
        }

        self.size = size;
        self.alloc_region = 0;
    }

    pub fn active_size(&self) -> usize {
        let mut size = 0;

        for old_region in &self.regions {
            size += old_region.active_size();
        }

        size
    }

    pub fn alloc(&mut self, config: &SharedHeapConfig, size: usize) -> Address {
        let ptr = self.pure_alloc(size);

        if ptr.is_non_null() {
            return ptr;
        }

        {
            let mut config = config.lock();

            if !config.grow_old(GEN_SIZE) {
                return Address::null();
            }
        }

        if !self.extend(GEN_SIZE) {
            return Address::null();
        }

        self.pure_alloc(size)
    }

    fn pure_alloc(&mut self, size: usize) -> Address {
        let alloc_region = self.alloc_region;

        if let Some(addr) = self.regions[alloc_region].pure_alloc(size) {
            return addr;
        }

        for (idx, old_region) in &mut self.regions.iter_mut().enumerate() {
            if let Some(addr) = old_region.pure_alloc(size) {
                self.alloc_region = idx;
                return addr;
            }
        }

        Address::null()
    }

    fn extend(&mut self, size: usize) -> bool {
        let alloc_region = self.alloc_region;

        if self.regions[alloc_region].extend(size) {
            self.size += size;
            return true;
        }

        for (idx, old_region) in &mut self.regions.iter_mut().enumerate() {
            if old_region.extend(size) {
                self.size += size;
                self.alloc_region = idx;
                return true;
            }
        }

        false
    }

    pub fn dump_regions(&self) {
        println!("OLD gen: {}", self.total);
        for (idx, region) in self.regions.iter().enumerate() {
            println!(
                "  OLD region {}: total={} committed={} active={}",
                idx,
                region.total_region(),
                region.committed_region(),
                region.active_region()
            );
        }
    }
}

#[derive(Clone)]
pub struct OldGenRegion {
    // Region boundaries. NOT necessarily page aligned!
    total: Region,

    // Maximum boundaries of mappings. Page aligned.
    total_mapping: Region,

    // Next object in this region is allocated here.
    top: Address,

    // Memory is mapped until here
    mapping_top: Address,
}

impl OldGenRegion {
    fn single(region: Region) -> OldGenRegion {
        assert!(region.start.is_page_aligned());

        OldGenRegion {
            total: region.clone(),
            top: region.start,
            total_mapping: region.clone(),
            mapping_top: region.start,
        }
    }

    pub fn new(
        total: Region,
        top: Address,
        total_mapping: Region,
        mapping_top: Address,
    ) -> OldGenRegion {
        assert!(total.valid_top(top));
        assert!(total_mapping.valid_top(mapping_top));

        OldGenRegion {
            total: total,
            total_mapping: total_mapping,
            top: top,
            mapping_top: mapping_top,
        }
    }

    pub fn start(&self) -> Address {
        self.total.start
    }

    pub fn top(&self) -> Address {
        self.top
    }

    pub fn size(&self) -> usize {
        self.total.size()
    }

    pub fn active_size(&self) -> usize {
        self.top.offset_from(self.total.start)
    }

    pub fn active_region(&self) -> Region {
        Region::new(self.total.start, self.top)
    }

    pub fn total_region(&self) -> Region {
        self.total.clone()
    }

    pub fn committed_region(&self) -> Region {
        Region::new(self.mapping_start(), self.mapping_end())
    }

    fn mapping_start(&self) -> Address {
        self.total_mapping.start
    }

    fn mapping_end(&self) -> Address {
        self.mapping_top
    }

    fn mapping_size(&self) -> usize {
        self.mapping_end().offset_from(self.mapping_start())
    }

    fn pure_alloc(&mut self, size: usize) -> Option<Address> {
        let new_top = self.top.offset(size);
        debug_assert!(self.top <= self.mapping_top);
        debug_assert!(self.top <= self.total.end);

        if new_top <= min(self.mapping_top, self.total.end) {
            let addr = self.top;
            self.top = new_top;
            return Some(addr);
        }

        None
    }

    fn extend(&mut self, size: usize) -> bool {
        let new_mapping_top = self.mapping_top.offset(size);

        if new_mapping_top <= self.total_mapping.end {
            arena::commit(self.mapping_top, size, false);
            self.mapping_top = new_mapping_top;

            true
        } else {
            false
        }
    }
}
