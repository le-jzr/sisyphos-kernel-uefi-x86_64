
// TODO
#![cfg(false)]

#[derive(Debug)]
pub struct BuddyAllocator<'a> {
    // Only useful for debug info.
    alloc_unit: usize,

    limit: usize,
    pivot: usize,
    pivot_rank: usize,
    root_rank: usize,

    bitmap_buffer: &'a mut [u64],
    tree_buffer: &'a mut [u64],
}

impl<'a> BuddyAllocator<'a> {
    const bitmap_rank = 9;

	const fn bitmap_size(address_limit: usize) -> usize {
		const CACHE_LINE_SIZE: usize = 64;
		return align_up(address_limit, CACHE_LINE_SIZE * 8) / 8;
	}

	const fn tree_size(address_limit: usize) -> usize {
		let address_limit = align_to_pow2(address_limit);
		let bitmap_segments = address_limit / 512;
		return bitmap_segments * mem::size_of<u64>() * 2;
	}

	pub fn new(address_limit: usize, bitmap_buffer: &'a mut [u64], tree_buffer: &'a mut [u64], alloc_unit: usize) -> Self {
	    let rrank = log2i(align_to_pow2(address_limit));
        tree_buffer[0] = 0;
        tree_buffer[1] = 0;

	    BuddyAllocator{
	        alloc_unit: alloc_unit,
	        limit: address_limit,
	        pivot: 1,
	        pivot_rank: rrank,
	        root_rank: rrank,
	        bitmap_buffer: bitmap_buffer,
	        tree_buffer: tree_buffer,
	    }
	}

	pub fn alloc(&mut self, size: usize) -> usize {
	    if (is_power_of_two(size)) {
		    return self.alloc_frame(log2i(size));
	    }

	    let rank = log2i(size)+1;
	    let frame = self.alloc_frame(rank);
	    // debug("Buddy allocator %x -- Allocated region %x of size %d, region size %d.", this, frame * _alloc_unit, size * _alloc_unit, (1<<rank) * _alloc_unit);
	    self.dealloc(frame + size, (1<<rank)-size);
	    frame
    }

    pub fn dealloc(offset: usize, size: usize) {
	    // debug("Buddy allocator %x -- Releasing region, offset %x, size %d.", this, offset * _alloc_unit, size * _alloc_unit);

	    while (size > 0) {
		    int rank = min(alignment_rank(offset), log2i(size));
		    free_frame(rank, offset);

		    offset += (1 << rank);
		    size -= (1 << rank);
	    }
    }

    // FIXME: Move somewhere else
    fn set_all(s: &mut [u64], val: u64) {
        // TODO: platform-optimized versions?
        for &mut x in s {
            *x = val;
        }
    }

    fn alloc_rank_0(bitmap: &mut [u64; 8]) -> usize {
       	for i in 0..8 {
		    if (bitmap[i] == 0) {
			    continue;
		    }

		    let offset = find_first_set(bitmap[i]);
		    bitmap[i] ^= (1 << offset);
		    return i*64 + offset;
	    }

	    unreachable!();
    }

    fn alloc_rank_1(bitmap: &mut [u64; 8]) -> usize {
	    for i in 0..8 {
		    let free_rank_1 = (bitmap[i] & (bitmap[i] >> 1) & 0x5555555555555555_u64);

		    if (free_rank_1 == 0) {
			    continue;
		    }

		    let offset = find_first_set(free_rank_1);
		    bitmap[i] ^= (0x3 << offset);
		    debug_assert!((bitmap[i] & (0x3 << offset)) == 0);
		    return i*64 + offset;
	    }

	    unreachable!();
    }

    fn alloc_rank_2(bitmap: &mut [u64; 8]) -> usize {
	    for i in 0..8 {
		    let free_rank_1 = (bitmap[i] & (bitmap[i] >> 1) & 0x5555555555555555_u64);
		    let free_rank_2 = (free_rank_1 & (free_rank_1 >> 2) & 0x1111111111111111_u64);

		    if (free_rank_2 == 0) {
			    continue;
		    }

		    let offset = find_first_set(free_rank_2);
		    bitmap[i] ^= (0xf << offset);
		    debug_assert!((bitmap[i] & (0xf << offset)) == 0);
		    return i*64 + offset;
	    }

	    unreachable!();
    }

    fn alloc_rank_3(bitmap: &mut [u64; 8]) -> usize {
	    for i in 0..8 {
	        let free_rank_1 = (bitmap[i] & (bitmap[i] >> 1) & 0x5555555555555555_u64);
		    let free_rank_2 = (free_rank_1 & (free_rank_1 >> 2) & 0x1111111111111111_u64);
		    let free_rank_3 = (free_rank_2 & (free_rank_2 >> 4) & 0x0101010101010101_u64);

            if (free_rank_3 == 0) {
			    continue;
		    }

		    let offset = find_first_set(free_rank_3);
		    bitmap[i] ^= (0xff << offset);
		    debug_assert!((bitmap[i] & (0xff << offset)) == 0);
		    return i*64 + offset;
	    }

	    unreachable!();
    }

    fn alloc_rank_4(bitmap: &mut [u64; 8]) -> usize {
	    for i in 0..8 {
	        let free_rank_1 = (bitmap[i] & (bitmap[i] >> 1) & 0x5555555555555555_u64);
		    let free_rank_2 = (free_rank_1 & (free_rank_1 >> 2) & 0x1111111111111111_u64);
		    let free_rank_3 = (free_rank_2 & (free_rank_2 >> 4) & 0x0101010101010101_u64);
            let free_rank_4 = (free_rank_3 & (free_rank_3 >> 8) & 0x0001000100010001_u64);

            if (free_rank_4 == 0) {
			    continue;
		    }

		    let offset = find_first_set(free_rank_4);
		    bitmap[i] ^= (0xffff << offset);
		    debug_assert!((bitmap[i] & (0xffff << offset)) == 0);
		    return i*64 + offset;
	    }

	    unreachable!();
    }

    fn alloc_rank_5(bitmap: &mut [u64; 8]) -> usize {
	    for i in 0..8 {
	        if (bitmap[i] & 0xffffffff) == 0xffffffff {
	            bitmap[i] ^= 0xffffffff;
	            return i*64;
	        }
	        if (bitmap[i] & 0xffffffff00000000) == 0xffffffff00000000 {
	            bitmap[i] ^= 0xffffffff00000000;
	            return i*64 + 32;
	        }
	    }

	    unreachable!();
    }

    fn alloc_rank_6(bitmap: &mut [u64; 8]) -> usize {
	    for i in 0..8 {
		    if (bitmap[i] == 0xffffffffffffffff_u64) {
			    bitmap[i] = 0;
			    return i*64;
		    }
	    }

	    unreachable!();
    }

    fn alloc_rank_7(bitmap: &mut [u64; 8]) -> usize {
	    for (int i = 0; i < 8; i+=2) {
		    if ((bitmap[i] & bitmap[i+1]) == 0xffffffffffffffff_u64) {
			    bitmap[i] = 0;
			    bitmap[i+1] = 0;
			    return i*64;
		    }
	    }

	    unreachable!();
    }

    fn alloc_rank_8(bitmap: &mut [u64; 8]) -> usize {
	    for (int i = 0; i < 8; i+=4) {
		    if ((bitmap[i] & bitmap[i+1] & bitmap[i+2] & bitmap[i+3]) != 0xffffffffffffffff_u64) {
			    continue;
		    }

		    bitmap[i] = 0;
		    bitmap[i+1] = 0;
		    bitmap[i+2] = 0;
		    bitmap[i+3] = 0;
		    return i*64;
	    }

	    unreachable!();
    }

    fn summarize_0(bitmap: u64) -> u64
    {
	    let summary = (bitmap != 0) as u64;

	    uint64_t rank_1 = (bitmap & (bitmap >> 1)) & 0x5555555555555555_u64;
	    summary |= ((rank_1 != 0) as u64 << 1);

	    uint64_t rank_2 = (rank_1 & (rank_1 >> 2)) & 0x1111111111111111_u64;
	    summary |= ((rank_2 != 0) as u64 << 2);

	    uint64_t rank_3 = (rank_2 & (rank_2 >> 4)) & 0x0101010101010101_u64;
	    summary |= ((rank_3 != 0) as u64 << 3);

	    uint64_t rank_4 = (rank_3 & (rank_3 >> 8)) & 0x0001000100010001_u64;
	    summary |= ((rank_4 != 0) as u64 << 4);

	    uint64_t rank_5 = (rank_4 & (rank_4 >> 16)) & 0x0000000100000001_u64;
	    summary |= ((rank_5 != 0) as u64 << 5);

	    uint64_t rank_6 = (rank_5 & (rank_5 >> 32)) & 0x0000000000000001_u64;
	    summary |= ((rank_6 != 0) as u64 << 6);

	    summary
    }

    fn summarize_1(bitmap: &[u64; 2]) -> u64
    {
	    let summary0 = summarize_0(bitmap[0]);
	    let summary1 = summarize_0(bitmap[1]);

	    if (summary0 & summary1 & (1<<6)) {
		    (1<<7)
	    } else {
            summary0 | summary1
        }
    }

    fn summarize_2(bitmap: &[u64; 4]) -> u64
    {
        let summary0 = summarize_1(&bitmap[0..2]);
        let summary1 = summarize_1(&bitmap[2..4]);

        if (summary0 & summary1 & (1<<7)) {
	        (1<<8)
        } else {
            summary0 | summary1
        }
    }

    fn summarize_3(bitmap: &[u64; 8]) -> u64
    {
	    let summary0 = summarize_2(&bitmap[0..4]);
	    let summary1 = summarize_2(&bitmap[4..8]);

	    if (summary0 & summary1 & (1<<8)) {
		    (1<<9)
	    } else {
            summary0 | summary1
        }
    }

    fn alloc_in_bitmap(bitmap: &[u64; 8], rank: usize) -> usize {
	    match rank {
	        0 => alloc_rank_0(bitmap),
	        1 => alloc_rank_1(bitmap),
	        2 => alloc_rank_2(bitmap),
	        3 => alloc_rank_3(bitmap),
	        4 => alloc_rank_4(bitmap),
	        5 => alloc_rank_5(bitmap),
	        6 => alloc_rank_6(bitmap),
	        7 => alloc_rank_7(bitmap),
	        8 => alloc_rank_8(bitmap),
	        _ => unreachable!(),
	    }
    }

    fn dealloc_in_bitmap(bitmap: &[u64; 8], rank: usize, offset: usize)
    {
	    let bitmap_offset = offset >> 6;

	    if (rank < 6) {
		    let mask = ((1_u64<<(1<<rank))-1) << (offset & 0x3f);
		    debug_assert!((bitmap[bitmap_offset] & mask) == 0);
		    bitmap[bitmap_offset] |= mask;
		    return;
	    }

	    match rank {
	    case 8:
		    assert(bitmap[bitmap_offset+2] == 0);
		    bitmap[bitmap_offset+2] = 0xffffffffffffffffull;
		    assert(bitmap[bitmap_offset+3] == 0);
		    bitmap[bitmap_offset+3] = 0xffffffffffffffffull;
	    case 7:
		    assert(bitmap[bitmap_offset+1] == 0);
		    bitmap[bitmap_offset+1] = 0xffffffffffffffffull;
	    case 6:
		    assert(bitmap[bitmap_offset] == 0);
		    bitmap[bitmap_offset] = 0xffffffffffffffffull;
		    return;
	    }

	    assert_not_reached();
    }

    fn search_up(&mut self, search_bit: u64) {
	    let pivot_rank = self.pivot_rank;
	    let pivot_rank_bit = 1_u64 << pivot_rank;
	    let pivot = self.pivot;
        let tree = self.tree_buffer;


	    while ((search_bit & tree[pivot]) == 0) {
		    debug_assert!(pivot > 1);

		    if (tree[pivot] == pivot_rank_bit && tree[pivot ^ 1] == pivot_rank_bit) {
			    // Coalesce.
			    tree[pivot/2] = pivot_rank_bit << 1;
		    } else {
		        // Aggregate.
			    tree[pivot/2] = tree[pivot] | tree[pivot ^ 1];
		    }

		    ++pivot_rank;
		    pivot_rank_bit <<= 1;
		    pivot /= 2;
	    }

	    self.pivot_rank = pivot_rank;
	    self.pivot = pivot;
    }

    fn search_down(search_bit: u64, rank: usize)
    {
	    let pivot_rank = self.pivot_rank;
	    let pivot_rank_bit = 1_u64 << pivot_rank;
	    let pivot = self.pivot;
	    let tree = self.tree_buffer;
        let bitmap_rank = self.bitmap_rank;


	    // Move pivot downward.
	    while (pivot_rank > bitmap_rank && pivot_rank > rank) {
		    if (tree[pivot] == pivot_rank_bit) {
			    // Split.
			    tree[2*pivot] = pivot_rank_bit >> 1;
			    tree[2*pivot+1] = pivot_rank_bit >> 1;

			    search_bit >>= 1;
		    }

		    pivot *= 2;

		    if (!(search_bit & tree[pivot])) {
			    // Switch to right branch if left does not have the right region.
			    ++pivot;
		    }

		    // Store summary of the upper parts of the tree.
		    tree[pivot/2] = tree[pivot/4] | tree[pivot^1];

		    --pivot_rank;
		    pivot_rank_bit >>= 1;
	    }

	    self.pivot_rank = pivot_rank;
	    self.pivot = pivot;
    }

    fn search_address_up(address: usize)
    {
	    let pivot_rank = self.pivot_rank;
	    let pivot_rank_bit = 1 << pivot_rank;
	    let pivot = self.pivot;

	    let paddress = pivot_address();

	    address >>= pivot_rank;
	    paddress >>= pivot_rank;

	    while (address != paddress) {
		    debug_assert!(pivot > 1);

		    if (tree[pivot] == pivot_rank_bit && tree[pivot ^ 1] == pivot_rank_bit) {
			    // Coalesce.
			    tree[pivot/2] = pivot_rank_bit << 1;
		    } else {
			    // Just summary.
			    tree[pivot/2] = tree[pivot] | tree[pivot ^ 1];
		    }

		    ++pivot_rank;
		    pivot_rank_bit <<= 1;
		    pivot /= 2;

		    address >>= 1;
		    paddress >>= 1;
	    }

	    self.pivot_rank = pivot_rank;
	    self.pivot = pivot;
    }

    fn search_address_down(address: usize, rank: usize)
    {
	    int pivot_rank = _pivot_rank;
	    uint64_t pivot_rank_bit = 1 << _pivot_rank;
	    int64_t pivot = _pivot;

	    while (pivot_rank > _bitmap_rank && pivot_rank > rank) {
		    if (_tree_base[pivot] == pivot_rank_bit) {
			    // Split.
			    _tree_base[2*pivot] = pivot_rank_bit >> 1;
			    _tree_base[2*pivot+1] = pivot_rank_bit >> 1;
		    }

		    if (_tree_base[pivot] == 0) {
			    // May be uninitialized.

			    _tree_base[2*pivot] = 0;
			    _tree_base[2*pivot+1] = 0;
		    }

		    pivot = 2*pivot + ((address >> (pivot_rank-1)) & 1);

		    // Store summary of the upper parts of the tree.
		    _tree_base[pivot/2] = _tree_base[pivot/4] | _tree_base[pivot^1];

		    --pivot_rank;
		    pivot_rank_bit >>= 1;
	    }

	    _pivot_rank = pivot_rank;
	    _pivot = pivot;
    }

    fn alloc_frame(rank: usize) -> usize
    {
	    if (rank > 63) {
		    // Request is bigger than the range of 64b integer.
		    return -1;
	    }

	    uint64_t rank_bit = 1 << rank;
	    uint64_t search_bit = rank_bit;

	    // Encodes availability of all orders in the memory.
	    uint64_t master_summary = _tree_base[_pivot] | _tree_base[_pivot/2];

	    if (rank_bit > master_summary) {
		    // Request is bigger than the largest free region.
		    return -1;
	    }

	    // Compute the smallest region that is big enough for the request.
	    while ((search_bit & master_summary) == 0) {
		    search_bit <<= 1;
	    }

	    // Search for the correct node.
	    search_up(search_bit);
	    search_down(search_bit, rank);

	    if (_pivot_rank == rank) {
		    // Allocated.
		    _tree_base[_pivot] = 0;
		    return pivot_address();
	    }

	    assert(_pivot_rank == _bitmap_rank);

	    // We have identified a 64-byte region of the bitmap.

	    int64_t address = pivot_address();

	    void* bitmap = &_bitmap_base[(address >> _bitmap_rank) * 8];

	    if (_tree_base[_pivot] == _bitmap_rank) {
		    memset64(bitmap, 0xffffffffffffffffull, 8);
	    }

	    int offset = alloc_in_bitmap(bitmap, rank);

	    // Update the tree.
	    _tree_base[_pivot] = summarize_3((aliasing_uint64_t*) bitmap);
	    return address + offset;
    }

    fn free_frame(rank: usize, address: usize)
    {
	    assert(rank < 64);

	    search_address_up(address);
	    search_address_down(address, rank);

	    assert(_pivot_rank == _bitmap_rank || _tree_base[_pivot] == 0);

	    if (_pivot_rank == rank) {
		    _tree_base[_pivot] = 1 << rank;
		    return;
	    }

	    assert(_pivot_rank == _bitmap_rank);
	    assert(rank < _bitmap_rank);

	    void* bitmap = &_bitmap_base[(address >> _bitmap_rank) * 8];

	    if (_tree_base[_pivot] == 0) {
		    memset64(bitmap, 0, 8);
	    }

	    dealloc_in_bitmap((aliasing_uint64_t*) bitmap, rank, address & ((1<<_bitmap_rank) - 1));

	    _tree_base[_pivot] = summarize_3((aliasing_uint64_t*) bitmap);
    }
}
