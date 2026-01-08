
use std::mem::MaybeUninit;
use smallvec::SmallVec;

enum Ports {
    Inline {
        len: u8,
        buf: MaybeUninit<[u16; 16]>
    },
    Heap(SmallVec<[u16; 16]>)

}

impl Ports {
    pub fn new() -> Self {
        Ports::Inline { len: 0, buf: MaybeUninit::uninit() }
    }

    pub fn push(&mut self, port: u16) {
        match self {
            Ports::Inline { len, buf } => {
                if (*len as usize) < 16 {
                    unsafe {
                        let ptr = buf.as_mut_ptr() as *mut u16;
                        ptr.add(*len as usize).write(port);
                    }
                    *len += 1;
                } else {
                    unsafe {
                        let old_buf = std::mem::replace(buf, MaybeUninit::uninit());
                        let old_len = *len as usize;


                        let mut n_v = SmallVec::<[u16; 16]>::from_buf_and_len_unchecked(old_buf, old_len);

                        n_v.push(port);

                        *self = Ports::Heap(n_v);
                    }
                    
                }
            },
            Ports::Heap(small_vec) => small_vec.push(port),
        }
    }

    pub fn as_slice(&self) -> &[u16] {
        match self {
            Ports::Inline { len, buf } => unsafe {
                std::slice::from_raw_parts(buf.as_ptr().cast::<u16>(), *len as usize)
            },
            Ports::Heap(v) => v.as_slice(),
        }
    }


}

impl Drop for Ports {
    fn drop(&mut self) {
        if let Ports::Inline { len, buf } = self {
            unsafe {
                let ptr = buf.as_mut_ptr() as *mut u16;
                for i in 0..(*len as usize) {
                    ptr.add(i).drop_in_place();
                }
            }
        }
    }
}