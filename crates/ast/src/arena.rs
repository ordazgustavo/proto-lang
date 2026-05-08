use std::marker::PhantomData;

pub trait ArenaId {
    type Id: Copy;

    fn new(idx: usize) -> Self::Id;
    fn index(id: Self::Id) -> usize;
}

#[derive(Debug)]
pub struct Id<T>(pub usize, pub PhantomData<T>);

impl<T> Copy for Id<T> {}

impl<T> Clone for Id<T> {
    #[inline]
    fn clone(&self) -> Id<T> {
        *self
    }
}

pub struct DefaultArenaId<T>(PhantomData<T>);

impl<T> ArenaId for DefaultArenaId<T> {
    type Id = Id<T>;

    fn new(idx: usize) -> Self::Id {
        Id(idx, PhantomData)
    }

    fn index(id: Self::Id) -> usize {
        id.0
    }
}

pub struct Arena<T, A = DefaultArenaId<T>> {
    data: Vec<T>,
    _id: PhantomData<A>,
}

impl<T, A> Arena<T, A>
where
    A: ArenaId,
{
    pub fn new() -> Self {
        Self {
            data: Vec::with_capacity(256),
            _id: PhantomData,
        }
    }

    pub fn alloc(&mut self, val: T) -> A::Id {
        let id = A::new(self.data.len());
        self.data.push(val);
        id
    }

    pub fn get(&self, id: A::Id) -> &T {
        &self.data[A::index(id)]
    }

    pub fn get_mut(&mut self, id: A::Id) -> &mut T {
        &mut self.data[A::index(id) as usize]
    }
}
