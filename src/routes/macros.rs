macro_rules! balanced_or_tree {
    ($x:expr $(,)?) => { debug_boxed!($x) };
    ($($x:expr),+ $(,)?) => {
        balanced_or_tree!(@internal ; $($x),+; $($x),+)
    };
    (@internal $($left:expr),*; $head:expr, $($tail:expr),+; $a:expr $(,$b:expr)?) => {
        (balanced_or_tree!($($left,)* $head)).or(balanced_or_tree!($($tail),+))
    };
    (@internal $($left:expr),*; $head:expr, $($tail:expr),+; $a:expr, $b:expr, $($more:expr),+) => {
        balanced_or_tree!(@internal $($left,)* $head; $($tail),+; $($more),+)
    };
}

#[cfg(debug_assertions)]
macro_rules! debug_boxed {
    ($x:expr) => {
        ::warp::Filter::boxed($x)
    };
}

#[cfg(not(debug_assertions))]
macro_rules! debug_boxed {
    ($x:expr) => {
        $x
    };
}