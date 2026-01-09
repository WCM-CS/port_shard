
use std::mem::MaybeUninit;
use smallvec::SmallVec;

#[derive(Debug)]
enum Chimera<T: PartialEq + Ord> {
    Inline {
        len: u8,
        buf: MaybeUninit<[T; 16]>
    },
    Heap(SmallVec<[T; 16]>)// can only store 32 values max

}

impl<T: PartialEq + Ord> Chimera<T> {
    pub fn new() -> Self {
        Chimera::Inline { len: 0, buf: MaybeUninit::uninit() }
    }

    pub fn from_vec(k: Vec<T>) -> Self {

        let n = k.len();

        if n <= 16 {
            let len = u8::try_from(n).unwrap();
            let mut chi: Chimera<T> = Chimera::Inline { len, buf: MaybeUninit::uninit() };

            if let Chimera::Inline { len: _, buf } = &mut chi {
               unsafe {
                    let ptr = buf.as_mut_ptr() as *mut T;
                    for (i, val) in k.into_iter().enumerate() {
                        ptr.add(i).write(val); // move into inline array
                    }
                }
            }

        
          //  k.into_iter().for_each(|k| chi.insert(k));
            chi
        } else {
            let mut iter = k.into_iter(); // shared iterator
            let mut sm: SmallVec<[T; 16]> = SmallVec::new();

            unsafe {
                let ptr = sm.as_mut_ptr();
                for i in 0..16 {
                    ptr.add(i).write(iter.next().unwrap());
                }

                for v in iter {
                    sm.push(v);
                }
            }

            Chimera::Heap(sm)
        }
    }



    pub fn insert(&mut self, port: T) {
        match self {
            Chimera::Inline { len, buf } => {
                let slice = unsafe { std::slice::from_raw_parts(buf.as_ptr().cast::<T>(), *len as usize) };
                if slice.contains(&port) {
                    return;
                }

                if (*len as usize) < 16 {
                    unsafe {
                        let ptr = buf.as_mut_ptr() as *mut T;
                        ptr.add(*len as usize).write(port);
                    }

                    *len += 1;
                } else {
                    unsafe {
                        let old_buf = std::mem::replace(buf, MaybeUninit::uninit());
                        let old_len = *len as usize;


                        let mut n_v = SmallVec::<[T; 16]>::from_buf_and_len_unchecked(old_buf, old_len);

                        n_v.push(port);
                        n_v.sort_unstable();

                        *self = Chimera::Heap(n_v);
                    }
                    
                }
            },
            Chimera::Heap(small_vec) => {
                match small_vec.binary_search(&port) {
                    Ok(_) => (),
                    Err(i) =>  small_vec.insert(i, port)
                }
            }
        }
    }

    pub fn contains(&self, port: &T) -> bool {
        match self {
            Chimera::Inline { len: _, buf: _ } => self.as_slice().iter().any(|k| k == port),
            Chimera::Heap(small_vec) => small_vec.binary_search(port).is_ok(),
        }
    }

 

    pub fn as_slice(&self) -> &[T] {
        match self {
            Chimera::Inline { len, buf } => unsafe {
                std::slice::from_raw_parts(buf.as_ptr().cast::<T>(), *len as usize)
            },
            Chimera::Heap(v) => v.as_slice(),
        }
    }

}




impl<T: PartialEq + Ord> Drop for Chimera<T> {
    fn drop(&mut self) {
        if let Chimera::Inline { len, buf } = self {
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
        let mut ports: Chimera<u16> = Chimera::new();

        match &ports {
            Chimera::Inline { len, .. } => assert_eq!(*len, 0),
            Chimera::Heap(_) => panic!("Expected Inline variant"),
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
            Chimera::Inline { len, .. } => assert_eq!(*len, 16),
            Chimera::Heap(_) => panic!("Expected Inline variant"),
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
            Chimera::Inline { .. } => panic!("Expected Heap variant"),
            Chimera::Heap(v) => {
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
        let ports: Chimera<u16> = Chimera::new();
        assert!(ports.as_slice().is_empty());
    }

    #[test]
    fn test_stack_drop_via_miri() {
        let mut ports = Chimera::new();

        ports.insert(3);
        ports.insert(7);
        ports.insert(88);
        assert!(!ports.as_slice().is_empty());


        

        // leak some memory for fun/miri testing
        //let s: String = String::from("I'm a leak");
        //let _ptr: *mut String = Box::into_raw(Box::new(s));

        //let ports_2: Chimera<String> = Chimera::new();
        let port_3: Chimera<[u16; 4]> = Chimera::new();


    }

    #[test] 
    fn test_from_vec() {

        let mut ports: Chimera<u16> = Chimera::from_vec(vec![1,2,3,4]);

        match &ports {
            Chimera::Inline { len, .. } => assert_eq!(ports.as_slice(), &[1,2,3,4]),
            Chimera::Heap(_) => panic!("Expected Inline variant"),
        }

        for i in 5..=17 {
            ports.insert(i);
        }


        match &ports {
            Chimera::Inline { .. } => panic!("Expected Heap variant"),
            Chimera::Heap(v) => {
                assert_eq!(v.len(), 17);
                assert_eq!(v.as_slice(), &[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17]);
            }
        }



        let mut ports_2: Chimera<u16> = Chimera::from_vec(vec![1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17]);

        match &ports {
            Chimera::Inline { .. } => panic!("Expected Heap variant"),
            Chimera::Heap(v) => {
                assert_eq!(v.len(), 17);
                assert_eq!(v.as_slice(), &[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17]);
            }
        }


    }
}