fn optimum_select<I, B, F, Cmp>(mut it: I, mut f: F, mut cmp: Cmp)
        -> Option<(I::Item, B)>
        where
            I: Iterator,
            F: FnMut(&I::Item) -> B,
            Cmp: FnMut(&I::Item, &B, &I::Item, &B) -> bool {
    it.next().map(|mut min| {
        let mut min_val = f(&min);
        for x in it {
            let x_val = f(&x);
            if cmp(&min, &min_val, &x, &x_val) {
                min_val = x_val;
                min = x;
            }
        }
        (min, min_val)
    })
}

// pub fn min_partial<I,>(it: I) -> Option<I::Item>
//         where I: Iterator, I::Item: PartialOrd {
//     optimum_select(it, |_| (), |m, _, x, _| m > x).map(|m| m.0)
// }

pub fn min_by_key_partial<I, B, F>(it: I, f: F) -> Option<I::Item>
        where I: Iterator, B: PartialOrd, F: FnMut(&I::Item) -> B {
    optimum_select(it, f, |_, m, _, x| m > x).map(|m| m.0)
}

pub fn max_partial<I,>(it: I) -> Option<I::Item>
        where I: Iterator, I::Item: PartialOrd {
    optimum_select(it, |_| (), |m, _, x, _| m < x).map(|m| m.0)
}

pub fn max_by_key_partial<I, B, F>(it: I, f: F) -> Option<I::Item>
        where I: Iterator, B: PartialOrd, F: FnMut(&I::Item) -> B {
    optimum_select(it, f, |_, m, _, x| m < x).map(|m| m.0)
}
