use std::alloc::{alloc, dealloc, Layout};
use std::fmt::{self, Debug};
use std::mem::{align_of, size_of};
use std::ptr::{self, null_mut, NonNull};

use log::debug;

use crate::level_data::level::Level;
use crate::p_ceiling::CeilingMove;
use crate::p_doors::VerticalDoor;
use crate::p_floor::FloorMove;
use crate::p_lights::{FireFlicker, Glow, LightFlash, StrobeFlash};
use crate::p_map_object::MapObject;
use crate::p_platforms::Platform;
use crate::p_player_sprite::PspDef;
use crate::player::Player;

#[derive(PartialEq, PartialOrd)]
pub struct TestObject {
    pub x: u32,
    pub thinker: NonNull<Thinker>,
}

impl Think for TestObject {
    fn think(object: &mut ThinkerType, _level: &mut Level) -> bool {
        let this = object.bad_mut::<TestObject>();
        this.x = 1000;
        true
    }

    fn set_thinker_ptr(&mut self, ptr: std::ptr::NonNull<Thinker>) {
        self.thinker = ptr;
    }

    fn thinker(&self) -> NonNull<Thinker> {
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
                object: ThinkerType::None,
                func: ActionF::None,
            })
        }

        Self {
            buf_ptr,
            capacity,
            len: 0,
            next_free: buf_ptr,
            tail: null_mut(),
        }
    }

    pub fn run_thinkers(&mut self, level: &mut Level) {
        let mut current = self.tail;
        let mut next;

        loop {
            unsafe {
                if (*current).remove() {
                    next = (*current).next;
                    self.remove(&mut *current);
                } else {
                    (*current).think(level);
                    next = (*current).next;
                }
            }
            current = next;

            if current == self.tail {
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
                if matches!((*ptr).func, ActionF::None)
                    && matches!((*ptr).object, ThinkerType::None)
                {
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
    pub fn push<T: Think>(&mut self, thinker: Thinker) -> Option<NonNull<Thinker>> {
        if self.len == self.capacity {
            return None;
        }
        if matches!(thinker.func, ActionF::None) {
            panic!("Can't push a thinker with ActionF::None as the function wrapper");
        }

        let root_ptr = self.find_first_free(self.next_free)?;
        debug!("Adding Thinker of type {:?}", thinker.object);
        unsafe {
            ptr::write(root_ptr, thinker);

            if self.tail.is_null() {
                self.tail = root_ptr;
                (*self.tail).prev = self.tail;
                (*self.tail).next = self.tail;
            } else {
                (*(*self.tail).prev).next = root_ptr;
                (*root_ptr).next = self.tail;
                (*root_ptr).prev = (*self.tail).prev;
                (*self.tail).prev = root_ptr;
            }

            (*root_ptr)
                .object
                .bad_mut::<T>()
                .set_thinker_ptr(NonNull::new_unchecked(root_ptr));

            self.len += 1;

            Some(NonNull::new_unchecked(root_ptr))
        }
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
        debug!("Removing Thinker of type {:?}", thinker.object);
        unsafe {
            thinker.func = ActionF::None;
            thinker.object = ThinkerType::None;
            (*thinker.next).prev = (*thinker).prev;
            (*thinker.prev).next = (*thinker).next;

            self.len -= 1;
            self.next_free = thinker; // reuse the slot on next insert
            self.maybe_reset_head();
        }
    }
}

/// All map object thinkers need to be registered here
#[repr(C)]
#[allow(clippy::large_enum_variant)]
pub enum ThinkerType {
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
    None,
}

impl Debug for ThinkerType {
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
            Self::None => f.debug_tuple("None - this shouldn't ever be seen").finish(),
        }
    }
}

impl ThinkerType {
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
    object: ThinkerType,
    func: ActionF,
}

impl Thinker {
    pub fn obj_ref(&self) -> &ThinkerType {
        &self.object
    }

    pub fn obj_mut(&mut self) -> &mut ThinkerType {
        &mut self.object
    }

    pub fn has_action(&self) -> bool {
        !matches!(self.func, ActionF::None)
    }

    pub fn remove(&self) -> bool {
        matches!(self.func, ActionF::Remove)
    }

    pub fn set_action(&mut self, func: ActionF) {
        self.func = func
    }

    /// Run the `ThinkerType`'s `think()`. If the `think()` returns false then the function pointer is set to None
    /// to mark removal.
    pub fn think(&mut self, level: &mut Level) -> bool {
        match self.func {
            ActionF::Action1(f) => (f)(&mut self.object, level),
            ActionF::Player(_f) => true,
            ActionF::None | ActionF::Test => true,
            ActionF::Remove => false,
        }
    }
}

impl fmt::Debug for Thinker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Thinker")
            .field("prev", &(self.prev as usize))
            .field("next", &(self.next as usize))
            .field("object", &(self as *const Self as usize))
            .field("func", &self.func)
            .finish()
    }
}

#[derive(Clone)]
pub enum ActionF {
    /// The slot in memory is "empty"
    None,
    /// To have the thinker removed from the thinker list on next cleanup pass
    Remove,
    /// Purely for testing without an action
    Test,
    Action1(fn(&mut ThinkerType, &mut Level) -> bool),
    Player(fn(&mut Player, &mut PspDef)), // P_SetPsprite runs this
}

impl fmt::Debug for ActionF {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActionF::Test => f.debug_struct("Test").finish(),
            ActionF::None => f.debug_struct("None").finish(),
            ActionF::Remove => f.debug_struct("Remove").finish(),
            ActionF::Action1(_) => f.debug_struct("Action1").finish(),
            ActionF::Player(_) => f.debug_struct("Player").finish(),
        }
    }
}

/// Every map object should implement this trait
pub trait Think {
    /// Creating a thinker should be the last step in new objects as `Thinker` takes ownership
    fn create_thinker(object: ThinkerType, func: ActionF) -> Thinker {
        Thinker {
            prev: null_mut(),
            next: null_mut(),
            object,
            func,
        }
    }

    fn state(&self) -> bool {
        if let ActionF::None = self.thinker_ref().func {
            return true;
        }
        false
    }

    /// impl of this trait function should return true *if* the thinker + object are to be removed
    ///
    /// Functionally this is Acp1, but in Doom when used with a Thinker it calls only one function
    /// on the object and Null is used to track if the map object should be removed.
    ///
    /// **NOTE:**
    ///
    /// The impl of `think()` on type will need to cast `ThinkerType` with `object.bad_mut()`.
    fn think(object: &mut ThinkerType, level: &mut Level) -> bool;

    /// Implementer must store the pointer to the conatining Thinker
    fn set_thinker_ptr(&mut self, ptr: NonNull<Thinker>);

    fn thinker(&self) -> NonNull<Thinker>;

    fn thinker_ref(&self) -> &Thinker {
        unsafe { self.thinker().as_ref() }
    }

    fn thinker_mut(&mut self) -> &mut Thinker {
        unsafe { self.thinker().as_mut() }
    }
}

#[cfg(test)]
mod tests {
    use wad::WadData;

    use crate::{
        d_thinker::{ActionF, TestObject, Think, Thinker, ThinkerType},
        level_data::{level::Level, map_data::MapData},
    };

    use super::ThinkerAlloc;
    use std::ptr::{null_mut, NonNull};

    #[test]
    fn bad_stuff() {
        let mut x = ThinkerType::Test(TestObject {
            x: 42,
            thinker: NonNull::dangling(),
        });

        if let ThinkerType::Test(f) = &x {
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

        let mut l = unsafe {
            Level::new(
                crate::d_main::Skill::Baby,
                1,
                1,
                crate::doom_def::GameMode::Shareware,
            )
        };
        let mut x = Thinker {
            prev: null_mut(),
            next: null_mut(),
            object: ThinkerType::Test(TestObject {
                x: 42,
                thinker: NonNull::dangling(),
            }),
            func: ActionF::Action1(TestObject::think),
        };

        assert!(x.think(&mut l));

        let ptr = NonNull::from(&mut x);
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

        let mut think = links
            .push::<TestObject>(TestObject::create_thinker(
                ThinkerType::Test(TestObject {
                    x: 42,
                    thinker: NonNull::dangling(),
                }),
                ActionF::Remove,
            ))
            .unwrap();
        assert!(!links.tail.is_null());
        assert_eq!(links.len(), 1);
        unsafe {
            assert_eq!((*links.tail).object.bad_ref::<TestObject>().x, 42);
        }

        unsafe {
            dbg!(&*links.buf_ptr.add(0));
            dbg!(&*links.buf_ptr.add(1));
            dbg!(&*links.buf_ptr.add(2));
            dbg!(&*links.buf_ptr.add(62));

            assert!(matches!((*links.buf_ptr.add(0)).func, ActionF::Remove));
            assert!(matches!((*links.buf_ptr.add(1)).func, ActionF::Remove));
            assert!(matches!((*links.buf_ptr.add(2)).func, ActionF::Remove));

            links.remove(think.as_mut());
            assert_eq!(links.len(), 0);
        }
    }

    #[test]
    fn check_next_prev_links() {
        let mut links = unsafe { ThinkerAlloc::new(64) };

        links
            .push::<TestObject>(TestObject::create_thinker(
                ThinkerType::Test(TestObject {
                    x: 42,
                    thinker: NonNull::dangling(),
                }),
                ActionF::None,
            ))
            .unwrap();
        assert!(!links.tail.is_null());

        let mut one = links
            .push::<TestObject>(TestObject::create_thinker(
                ThinkerType::Test(TestObject {
                    x: 666,
                    thinker: NonNull::dangling(),
                }),
                ActionF::None,
            ))
            .unwrap();

        links
            .push::<TestObject>(TestObject::create_thinker(
                ThinkerType::Test(TestObject {
                    x: 123,
                    thinker: NonNull::dangling(),
                }),
                ActionF::None,
            ))
            .unwrap();
        let mut three = links
            .push::<TestObject>(TestObject::create_thinker(
                ThinkerType::Test(TestObject {
                    x: 333,
                    thinker: NonNull::dangling(),
                }),
                ActionF::None,
            ))
            .unwrap();

        unsafe {
            // forward
            assert_eq!((*links.buf_ptr).object.bad_ref::<TestObject>().x, 42);
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
            assert_eq!(
                (*(*(*(*(*links.buf_ptr).next).next).next).next)
                    .object
                    .bad_ref::<TestObject>()
                    .x,
                42
            );
            // back
            assert_eq!((*links.tail).object.bad_ref::<TestObject>().x, 42);
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
            links.remove(one.as_mut());
            assert_eq!((*links.tail).object.bad_ref::<TestObject>().x, 42);
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
            links.remove(three.as_mut());
            assert_eq!((*links.tail).object.bad_ref::<TestObject>().x, 42);
            assert_eq!((*(*links.tail).prev).object.bad_ref::<TestObject>().x, 123);
            assert_eq!(
                (*(*(*links.tail).prev).prev)
                    .object
                    .bad_ref::<TestObject>()
                    .x,
                42
            );
        }
    }

    // #[test]
    // fn link_iter_and_removes() {
    //     let mut links = unsafe { ThinkerAlloc::new(64) };

    //     links.push::<TestObject>(TestObject::create_thinker(
    //         ThinkerType::Test(TestObject {
    //             x: 42,
    //             thinker: NonNull::dangling(),
    //         }),
    //         ActionF::None,
    //     ));
    //     links.push::<TestObject>(TestObject::create_thinker(
    //         ThinkerType::Test(TestObject {
    //             x: 123,
    //             thinker: NonNull::dangling(),
    //         }),
    //         ActionF::None,
    //     ));
    //     links.push::<TestObject>(TestObject::create_thinker(
    //         ThinkerType::Test(TestObject {
    //             x: 666,
    //             thinker: NonNull::dangling(),
    //         }),
    //         ActionF::None,
    //     ));
    //     links.push::<TestObject>(TestObject::create_thinker(
    //         ThinkerType::Test(TestObject {
    //             x: 333,
    //             thinker: NonNull::dangling(),
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
