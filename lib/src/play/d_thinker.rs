use std::alloc::{alloc, dealloc, Layout};
use std::fmt::{self, Debug};
use std::marker::PhantomPinned;
use std::mem::{align_of, size_of};
use std::ptr::{self, null_mut};

use log::debug;

use super::{
    ceiling::CeilingMove,
    doors::VerticalDoor,
    floor::FloorMove,
    lights::{FireFlicker, Glow, LightFlash, StrobeFlash},
    map_object::MapObject,
    platforms::Platform,
};
use crate::level_data::Level;

#[derive(PartialEq, PartialOrd)]
pub struct TestObject {
    pub x: u32,
    pub thinker: *mut Thinker,
}

impl Think for TestObject {
    fn think(object: &mut ObjectType, _level: &mut Level) -> bool {
        let this = object.bad_mut::<TestObject>();
        this.x = 1000;
        true
    }

    fn set_thinker_ptr(&mut self, ptr: *mut Thinker) {
        self.thinker = ptr;
    }

    fn thinker(&self) -> *mut Thinker {
        self.thinker
    }
}

/// A custom allocation for `Thinker` objects. This intends to keep them in a contiguous
/// zone of memory.
pub struct ThinkerAlloc {
    /// The main AllocPool buffer
    buf_ptr: *mut Thinker,
    /// Total capacity. Not possible to allocate over this.
    capacity: usize,
    /// Actual used AllocPool
    len: usize,
    /// The next free slot to insert in
    next_free: *mut Thinker,
    pub tail: *mut Thinker,
    _pinned: PhantomPinned,
}

impl Drop for ThinkerAlloc {
    fn drop(&mut self) {
        unsafe {
            for idx in 0..self.capacity {
                self.drop_item(idx);
            }
            let size = self.capacity * size_of::<Thinker>();
            let layout = Layout::from_size_align_unchecked(size, align_of::<Thinker>());
            dealloc(self.buf_ptr as *mut _, layout);
        }
    }
}

impl ThinkerAlloc {
    /// # Safety
    /// Once allocated the owner of this `ThinkerAlloc` must not move.
    pub unsafe fn new(capacity: usize) -> Self {
        let size = capacity * size_of::<Thinker>();
        let layout = Layout::from_size_align_unchecked(size, align_of::<Thinker>());
        let buf_ptr = alloc(layout) as *mut Thinker;

        // Need to initialise everything to a blank slate
        for n in 0..capacity {
            buf_ptr.add(n).write(Thinker {
                prev: null_mut(),
                next: null_mut(),
                object: ObjectType::Free,
                func: Thinker::placeholder,
            })
        }

        Self {
            buf_ptr,
            capacity,
            len: 0,
            next_free: buf_ptr,
            tail: null_mut(),
            _pinned: PhantomPinned,
        }
    }

    pub fn run_thinkers(&mut self, level: &mut Level) {
        let mut current = unsafe { &mut *self.tail };
        let mut next;

        loop {
            unsafe {
                if current.should_remove() {
                    next = &mut *current.next;
                    self.remove(&mut *current);
                } else {
                    current.think(level);
                    next = &mut *current.next;
                }
            }
            current = next;

            if ptr::eq(current, self.tail) {
                return;
            }
        }
    }

    unsafe fn drop_item(&mut self, idx: usize) {
        debug_assert!(idx < self.capacity);
        let ptr = self.ptr_for_idx(idx);
        if std::mem::needs_drop::<Thinker>() {
            ptr::drop_in_place(ptr);
        }
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    fn ptr_for_idx(&self, idx: usize) -> *mut Thinker {
        unsafe { self.buf_ptr.add(idx) }
    }

    fn find_first_free(&self, mut ptr: *mut Thinker) -> Option<*mut Thinker> {
        if self.len >= self.capacity {
            return None;
        }

        loop {
            unsafe {
                if matches!((*ptr).object, ObjectType::Free) {
                    return Some(ptr);
                }
                ptr = ptr.add(1);
            }
            if ptr == self.buf_ptr {
                break;
            }
        }

        panic!("No more thnker slots");
    }

    /// Push an item to the `ThinkerAlloc`. Returns the pointer to the Thinker.
    ///
    /// # Safety:
    ///
    /// `<T>` must match the inner type of `Thinker`
    pub fn push<T: Think>(&mut self, thinker: Thinker) -> Option<*mut Thinker> {
        if self.len == self.capacity {
            return None;
        }
        if matches!(thinker.object, ObjectType::Free) {
            panic!("Can't push a thinker with ActionF::Free as the function wrapper");
        }

        let root_ptr = self.find_first_free(self.next_free)?;
        debug!("Adding Thinker of type {:?}", thinker.object);
        unsafe { ptr::write(root_ptr, thinker) };
        let mut current = unsafe { &mut *root_ptr };

        if self.tail.is_null() {
            self.tail = root_ptr;
            let mut tail = unsafe { &mut *self.tail };
            tail.prev = tail;
            tail.next = tail;
        } else {
            let mut tail = unsafe { &mut *self.tail };
            unsafe {
                (*tail.prev).next = root_ptr;
            }
            current.next = tail;
            current.prev = tail.prev;
            tail.prev = root_ptr;
        }

        current.object.bad_mut::<T>().set_thinker_ptr(root_ptr);
        self.len += 1;
        Some(root_ptr)
    }

    /// Ensure head is null if the pool is zero length
    fn maybe_reset_head(&mut self) {
        if self.len == 0 {
            self.tail = null_mut();
        }
    }

    /// Removes the entry at index. Sets both func + object to None values to indicate
    /// the slot is "empty".
    pub fn remove(&mut self, thinker: &mut Thinker) {
        debug!("Removing Thinker: {:?}", thinker);
        unsafe {
            thinker.object = ObjectType::Free;
            (*thinker.next).prev = (*thinker).prev;
            (*thinker.prev).next = (*thinker).next;

            self.len -= 1;
            self.next_free = thinker; // reuse the slot on next insert
            self.maybe_reset_head();
        }
    }
}

/// Thinkers *must* be contained in a structure that has **stable** memory locations.
/// In Doom this is managed by Doom's custom allocator `z_malloc`, where each location in memory
/// also has a pointer to the locations 'owner'. When Doom does a defrag or any op
/// that moves memory locations it also runs through the owners and updates their
/// pointers. This isn't done in the Rust version as that introduces a lot of overhead
/// and makes various things harder to do or harder to prove correct (if using unsafe).
///
/// Another way to manager Thinkers in a volatile container like a Vec is to use `self.function`
/// to mark for removal (same as Doom), then iterate over the container and only
/// run thinkers not marked for removal, then remove marked thinkers after cycle.
/// This method would have a big impact on iter speed though as there may be many
/// 'dead' thinkers and it would also impact the order of thinkers, which then means
/// recorded demo playback may be quite different to OG Doom.
///
/// Inserting the `Thinker` in to the game is done in p_tick.c with `P_RunThinkers`.
///
/// The LinkedList style serves to give the Objects a way to find the next/prev of
/// its neighbours and more, without having to pass in a ref to the Thinker container,
/// or iterate over possible blank spots in memory.
pub struct Thinker {
    prev: *mut Thinker,
    next: *mut Thinker,
    object: ObjectType,
    func: fn(&mut ObjectType, &mut Level) -> bool,
}

impl Thinker {
    pub fn obj_type(&self) -> &ObjectType {
        &self.object
    }

    pub fn obj_ref<T: Think>(&self) -> &T {
        self.object.bad_ref::<T>()
    }

    pub fn obj_mut<T: Think>(&mut self) -> &mut T {
        self.object.bad_mut::<T>()
    }

    pub fn should_remove(&self) -> bool {
        matches!(self.object, ObjectType::Remove)
    }

    pub fn mark_remove(&mut self) {
        self.object = ObjectType::Remove;
    }

    pub fn set_action(&mut self, func: fn(&mut ObjectType, &mut Level) -> bool) {
        self.func = func
    }

    pub fn set_obj_thinker_ptr<T: Think>(&mut self, ptr: *mut Thinker) {
        self.object.bad_mut::<T>().set_thinker_ptr(ptr);
    }

    /// Run the `ThinkerType`'s `think()`. If the `think()` returns false then it should be
    /// marked for removal
    pub fn think(&mut self, level: &mut Level) -> bool {
        (self.func)(&mut self.object, level)
    }

    /// Empty function purely for ThinkerAlloc init
    fn placeholder(_: &mut ObjectType, _: &mut Level) -> bool {
        false
    }
}

impl fmt::Debug for Thinker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Thinker")
            .field("prev", &(self.prev))
            .field("next", &(self.next))
            .field("object", &(self as *const Self))
            .finish_non_exhaustive()
    }
}

/// Every map object should implement this trait
pub trait Think {
    /// Creating a thinker should be the last step in new objects as `Thinker` takes ownership
    fn create_thinker(
        object: ObjectType,
        func: fn(&mut ObjectType, &mut Level) -> bool,
    ) -> Thinker {
        Thinker {
            prev: null_mut(),
            next: null_mut(),
            object,
            func,
        }
    }

    fn set_thinker_action(&mut self, func: fn(&mut ObjectType, &mut Level) -> bool) {
        self.thinker_mut().func = func
    }

    /// impl of this trait function should return true *if* the thinker + object are to be removed
    ///
    /// Functionally this is Acp1, but in Doom when used with a Thinker it calls only one function
    /// on the object and Null is used to track if the map object should be removed.
    ///
    /// **NOTE:**
    ///
    /// The impl of `think()` on type will need to cast `ThinkerType` with `object.bad_mut()`.
    fn think(object: &mut ObjectType, level: &mut Level) -> bool;

    /// Implementer must store the pointer to the conatining Thinker
    fn set_thinker_ptr(&mut self, ptr: *mut Thinker);

    fn thinker(&self) -> *mut Thinker;

    fn thinker_ref(&self) -> &Thinker {
        unsafe { &*self.thinker() }
    }

    fn thinker_mut(&mut self) -> &mut Thinker {
        unsafe { &mut *self.thinker() }
    }
}

/// All map object thinkers need to be registered here. If the object has pointees then these must be dealt
/// with before setting `ObjectType::Remove`.
#[repr(C)]
#[allow(clippy::large_enum_variant)]
pub enum ObjectType {
    Test(TestObject),
    Mobj(MapObject),
    VDoor(VerticalDoor),
    FloorMove(FloorMove),
    CeilingMove(CeilingMove),
    Platform(Platform),
    LightFlash(LightFlash),
    StrobeFlash(StrobeFlash),
    FireFlicker(FireFlicker),
    Glow(Glow),
    /// The thinker function should set to this when the linked-list node
    /// and memory is no-longer required. On thinker run it will be set to
    /// `Free` and unlinked.
    Remove,
    /// Used to mark a `ThinkerAlloc` slot as free to be re-used.
    Free,
}

impl Debug for ObjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Test(_) => f.debug_tuple("Test").finish(),
            Self::Mobj(_) => f.debug_tuple("Mobj").finish(),
            Self::VDoor(_) => f.debug_tuple("VDoor").finish(),
            Self::FloorMove(_) => f.debug_tuple("FloorMove").finish(),
            Self::CeilingMove(_) => f.debug_tuple("CeilingMove").finish(),
            Self::Platform(_) => f.debug_tuple("Platform").finish(),
            Self::LightFlash(_) => f.debug_tuple("LightFlash").finish(),
            Self::StrobeFlash(_) => f.debug_tuple("StrobeFlash").finish(),
            Self::FireFlicker(_) => f.debug_tuple("FireFlicker").finish(),
            Self::Glow(_) => f.debug_tuple("Glow").finish(),
            Self::Remove => f.debug_tuple("Remove").finish(),
            Self::Free => f.debug_tuple("Free - this shouldn't ever be seen").finish(),
        }
    }
}

impl ObjectType {
    pub fn bad_ref<T>(&self) -> &T {
        let mut ptr = self as *const Self as usize;
        ptr += size_of::<u64>();
        unsafe { &*(ptr as *const T) }
    }

    pub fn bad_mut<T>(&mut self) -> &mut T {
        let mut ptr = self as *mut Self as usize;
        ptr += size_of::<u64>();
        unsafe { &mut *(ptr as *mut T) }
    }
}

#[cfg(test)]
mod tests {
    use wad::WadData;

    use crate::{
        d_main::Skill,
        doom_def::GameMode,
        level_data::{map_data::MapData, Level},
        play::d_thinker::{Think, Thinker},
    };

    use super::{ObjectType, TestObject, ThinkerAlloc};
    use std::ptr::null_mut;

    #[test]
    fn bad_stuff() {
        let mut x = ObjectType::Test(TestObject {
            x: 42,
            thinker: null_mut(),
        });

        if let ObjectType::Test(f) = &x {
            assert_eq!(f.x, 42);

            let f = x.bad_ref::<TestObject>();
            assert_eq!(f.x, 42);

            assert_eq!(x.bad_mut::<TestObject>().x, 42);

            x.bad_mut::<TestObject>().x = 55;
            assert_eq!(x.bad_mut::<TestObject>().x, 55);
        }
    }

    #[test]
    fn bad_stuff_thinking() {
        let wad = WadData::new("../doom1.wad".into());
        let mut map = MapData::new("E1M1".to_owned());
        map.load(&wad);

        let mut l = unsafe { Level::new(Skill::Baby, 1, 1, GameMode::Shareware) };
        let mut x = Thinker {
            prev: null_mut(),
            next: null_mut(),
            object: ObjectType::Test(TestObject {
                x: 42,
                thinker: null_mut(),
            }),
            func: TestObject::think,
        };

        assert!(x.think(&mut l));

        let ptr = &mut x as *mut Thinker;
        x.object.bad_mut::<TestObject>().set_thinker_ptr(ptr);
        assert!(x.object.bad_mut::<TestObject>().thinker_mut().think(&mut l));
    }

    #[test]
    fn allocate() {
        let links = unsafe { ThinkerAlloc::new(64) };
        assert_eq!(links.len(), 0);
        assert_eq!(links.capacity(), 64);
    }

    #[test]
    fn push_1() {
        let mut links = unsafe { ThinkerAlloc::new(64) };
        assert_eq!(links.len(), 0);
        assert_eq!(links.capacity(), 64);

        let think = links
            .push::<TestObject>(TestObject::create_thinker(
                ObjectType::Remove,
                TestObject::think,
            ))
            .unwrap();
        assert!(!links.tail.is_null());
        assert_eq!(links.len(), 1);
        unsafe {
            assert!((*links.tail).should_remove());
        }

        unsafe {
            dbg!(&*links.buf_ptr.add(0));
            dbg!(&*links.buf_ptr.add(1));
            dbg!(&*links.buf_ptr.add(2));
            dbg!(&*links.buf_ptr.add(62));

            assert!(matches!((*links.buf_ptr.add(0)).object, ObjectType::Remove));
            assert!(matches!((*links.buf_ptr.add(1)).object, ObjectType::Free));
            assert!(matches!((*links.buf_ptr.add(2)).object, ObjectType::Free));

            links.remove(&mut *think);
            assert_eq!(links.len(), 0);
        }
    }

    #[test]
    fn check_next_prev_links() {
        let mut links = unsafe { ThinkerAlloc::new(64) };

        links
            .push::<TestObject>(TestObject::create_thinker(
                ObjectType::Remove,
                TestObject::think,
            ))
            .unwrap();
        assert!(!links.tail.is_null());

        let one = links
            .push::<TestObject>(TestObject::create_thinker(
                ObjectType::Test(TestObject {
                    x: 666,
                    thinker: null_mut(),
                }),
                TestObject::think,
            ))
            .unwrap();

        links
            .push::<TestObject>(TestObject::create_thinker(
                ObjectType::Test(TestObject {
                    x: 123,
                    thinker: null_mut(),
                }),
                TestObject::think,
            ))
            .unwrap();
        let three = links
            .push::<TestObject>(TestObject::create_thinker(
                ObjectType::Test(TestObject {
                    x: 333,
                    thinker: null_mut(),
                }),
                TestObject::think,
            ))
            .unwrap();

        unsafe {
            // forward
            assert!((*links.buf_ptr).should_remove());
            assert_eq!(
                (*(*links.buf_ptr).next).object.bad_ref::<TestObject>().x,
                666
            );
            assert_eq!(
                (*(*(*links.buf_ptr).next).next)
                    .object
                    .bad_ref::<TestObject>()
                    .x,
                123
            );
            assert_eq!(
                (*(*(*(*links.buf_ptr).next).next).next)
                    .object
                    .bad_ref::<TestObject>()
                    .x,
                333
            );
            assert!((*(*(*(*(*links.buf_ptr).next).next).next).next).should_remove());
            // back
            assert!((*links.tail).should_remove());
            assert_eq!((*(*links.tail).prev).object.bad_ref::<TestObject>().x, 333);
            assert_eq!(
                (*(*(*links.tail).prev).prev)
                    .object
                    .bad_ref::<TestObject>()
                    .x,
                123
            );
            assert_eq!(
                (*(*(*(*links.tail).prev).prev).prev)
                    .object
                    .bad_ref::<TestObject>()
                    .x,
                666
            );
        }
        unsafe {
            links.remove(&mut *one);
            assert!((*links.tail).should_remove());
            assert_eq!((*(*links.tail).prev).object.bad_ref::<TestObject>().x, 333);
            assert_eq!(
                (*(*(*links.tail).prev).prev)
                    .object
                    .bad_ref::<TestObject>()
                    .x,
                123
            );
        }

        unsafe {
            links.remove(&mut *three);
            assert!((*links.tail).should_remove());
            assert_eq!((*(*links.tail).prev).object.bad_ref::<TestObject>().x, 123);
            assert!((*(*(*links.tail).prev).prev).should_remove());
        }
    }

    // #[test]
    // fn link_iter_and_removes() {
    //     let mut links = unsafe { ThinkerAlloc::new(64) };

    //     links.push::<TestObject>(TestObject::create_thinker(
    //         ThinkerType::Test(TestObject {
    //             x: 42,
    //             thinker: null_mut(),
    //         }),
    //         ActionF::None,
    //     ));
    //     links.push::<TestObject>(TestObject::create_thinker(
    //         ThinkerType::Test(TestObject {
    //             x: 123,
    //             thinker: null_mut(),
    //         }),
    //         ActionF::None,
    //     ));
    //     links.push::<TestObject>(TestObject::create_thinker(
    //         ThinkerType::Test(TestObject {
    //             x: 666,
    //             thinker: null_mut(),
    //         }),
    //         ActionF::None,
    //     ));
    //     links.push::<TestObject>(TestObject::create_thinker(
    //         ThinkerType::Test(TestObject {
    //             x: 333,
    //             thinker: null_mut(),
    //         }),
    //         ActionF::None,
    //     ));

    //     for (i, thinker) in links.iter().enumerate() {
    //         if i == 0 {
    //             assert_eq!(thinker.object.bad_ref::<TestObject>().x, 42);
    //         }
    //         if i == 1 {
    //             assert_eq!(thinker.object.bad_ref::<TestObject>().x, 123);
    //         }
    //         if i == 2 {
    //             assert_eq!(thinker.object.bad_ref::<TestObject>().x, 666);
    //         }
    //         if i == 3 {
    //             assert_eq!(thinker.object.bad_ref::<TestObject>().x, 333);
    //         }
    //     }
    //     unsafe {
    //         assert_eq!(
    //             (*links.buf_ptr.add(3))
    //                 .as_ref()
    //                 .unwrap()
    //                 .object
    //                 .bad_ref::<TestObject>()
    //                 .x,
    //             333
    //         );
    //     }

    //     assert_eq!(links.iter().count(), 4);

    //     links.remove(3);
    //     assert_eq!(links.len(), 3);
    //     assert_eq!(links.iter().count(), 3);

    //     for (i, num) in links.iter().enumerate() {
    //         if i == 0 {
    //             assert_eq!((*num).object.bad_ref::<TestObject>().x, 42);
    //         }
    //         if i == 1 {
    //             assert_eq!((*num).object.bad_ref::<TestObject>().x, 123);
    //         }
    //         if i == 2 {
    //             assert_eq!((*num).object.bad_ref::<TestObject>().x, 666);
    //         }
    //     }
    //     //
    //     links.remove(1);
    //     assert_eq!(links.len(), 2);
    //     assert_eq!(links.iter().count(), 2);

    //     for (i, num) in links.iter().enumerate() {
    //         if i == 0 {
    //             assert_eq!((*num).object.bad_ref::<TestObject>().x, 42);
    //         }
    //         if i == 1 {
    //             assert_eq!((*num).object.bad_ref::<TestObject>().x, 666);
    //         }
    //     }
    // }
}
