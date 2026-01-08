
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

    pub fn insert(&mut self, port: u16) {
        match self {
            Ports::Inline { len, buf } => {
                let slice = unsafe { std::slice::from_raw_parts(buf.as_ptr().cast::<u16>(), *len as usize) };
                if slice.contains(&port) {
                    return;
                }

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
                        n_v.sort_unstable();

                        *self = Ports::Heap(n_v);
                    }
                    
                }
            },
            Ports::Heap(small_vec) => {
                match small_vec.binary_search(&port) {
                    Ok(_) => return,
                    Err(i) =>  small_vec.insert(i, port)
                }
            }
        }
    }

    pub fn contains(&self, port: &u16) -> bool {
        match self {
            Ports::Inline { len, buf } => self.as_slice().iter().any(|k| k == port),
            Ports::Heap(small_vec) => small_vec.binary_search(&port).is_ok(),
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


#[cfg(test)]
mod tests {
    use super::*;
    use smallvec::smallvec;

    #[test]
    fn test_ports_push_and_as_slice() {
        let mut ports = Ports::new();

        match &ports {
            Ports::Inline { len, .. } => assert_eq!(*len, 0),
            Ports::Heap(_) => panic!("Expected Inline variant"),
        }

        for i in 1..=16 {
            ports.insert(i);
        }

        // check stack
        let slice = ports.as_slice();
        assert_eq!(slice.len(), 16);
        for i in 0..16 {
            assert_eq!(slice[i], (i + 1) as u16);
        }

        match &ports {
            Ports::Inline { len, .. } => assert_eq!(*len, 16),
            Ports::Heap(_) => panic!("Expected Inline variant"),
        }

        
         // spill to heap, note original 16 ports stay stack allocated when moved to heap enum varaint, only 16+ get heap allocated
        ports.insert(17);

        // Check heap storage
        let slice = ports.as_slice();
        assert_eq!(slice.len(), 17);
        for i in 0..17 {
            assert_eq!(slice[i], (i + 1) as u16);
        }

        match &ports {
            Ports::Inline { .. } => panic!("Expected Heap variant"),
            Ports::Heap(v) => {
                assert_eq!(v.len(), 17);
                assert_eq!(v.as_slice(), &[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17]);
            }
        }

        // Push a few more ports
        ports.insert(18);
        ports.insert(19);

        let slice = ports.as_slice();
        assert_eq!(slice, &[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19]);
        assert!(slice.contains(&8));
        assert!(!slice.contains(&30));


        assert!(ports.contains(&8));
        assert!(!ports.contains(&30));



        ports.insert(7);
        assert_eq!(ports.as_slice().len(), 19);

        ports.insert(33);
        assert_eq!(ports.as_slice().len(), 20);

        let slice = ports.as_slice();
        assert_eq!(slice, &[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,33]);


    }

    #[test]
    fn test_empty_ports() {
        let ports = Ports::new();
        assert!(ports.as_slice().is_empty());
    }
}