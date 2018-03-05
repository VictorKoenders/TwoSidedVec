#[macro_use]
extern crate two_sided_vec;
extern crate serde;
extern crate serde_test;

use std::fmt::Debug;

use serde_test::{Token};
use two_sided_vec::{TwoSidedVec, TwoSidedExtend};

#[test]
fn test_push_front() {
    let mut result = TwoSidedVec::new();
    let expected = expected_front();
    for &i in &expected {
        result.push_front(i);
    }
    assert_expected(&result, vec![], expected);
}
#[test]
fn test_push_back() {
    let mut result = TwoSidedVec::new();
    let expected = expected_back();
    for &i in &expected {
        result.push_back(i);
    }
    assert_expected(&result, expected, vec![]);
}
#[test]
fn test_push() {
    let mut result = TwoSidedVec::new();
    let expected_back = expected_back();
    let expected_front = expected_front();
    for &i in &expected_back {
        result.push_back(i);
    }
    for &i in &expected_front {
        result.push_front(i);
    }
    assert_expected(&result, expected_back, expected_front);
}
#[test]
fn test_pop() {
    let mut result = TwoSidedVec::new();
    let mut expected_back = expected_back();
    let mut expected_front = expected_front();
    for &i in &expected_back {
        result.push_back(i);
    }
    for &i in &expected_front {
        result.push_front(i);
    }
    assert_expected(
        &result,
        expected_back.clone(),
        expected_front.clone()
    );
    while let Some(expected) = expected_back.pop() {
        assert_eq!(expected, result.pop_back().unwrap());
    }
    assert_eq!(result.len_back(), expected_back.len());
    while let Some(expected) = expected_front.pop() {
        assert_eq!(expected, result.pop_front().unwrap());
    }
    assert_eq!(result.len_front(), expected_front.len());
    assert!(result.is_empty());
}
#[test]
fn test_serde() {
    let values = two_sided_vec![1, 2, 3; 7, 8, 9, 10];
    ::serde_test::assert_tokens(&values, &[
        Token::Struct {
            name: "TwoSidedVec",
            len: 2
        },
        Token::Str("back"),
        Token::Seq { len: Some(3) },
        Token::I32(3),
        Token::I32(2),
        Token::I32(1),
        Token::SeqEnd,
        Token::Str("front"),
        Token::Seq { len: Some(4) },
        Token::I32(7),
        Token::I32(8),
        Token::I32(9),
        Token::I32(10),
        Token::SeqEnd,
        Token::StructEnd
    ])
}
#[test]
fn test_extend_ref_slice() {
    let mut result = TwoSidedVec::<u32>::new();
    result.extend_back(&[3, 2, 1]);
    result.extend_front(&[4, 5, 6]);
    assert_eq!(result.back(), &[1, 2, 3]);
    assert_eq!(result.front(), &[4, 5, 6])
}

#[test]
fn test_extend_ref_trusted_len() {
    let mut result = TwoSidedVec::<u32>::new();
    result.extend_back([3, 2, 1].iter().map(|i| i));
    result.extend_front([4, 5, 6].iter().map(|i| i));
    assert_eq!(result.back(), &[1, 2, 3]);
    assert_eq!(result.front(), &[4, 5, 6])
}

#[test]
fn test_extend_ref_default() {
    let mut result = TwoSidedVec::<u32>::new();
    result.extend_back([3, 2, 1].iter().map(|i| i).filter(|i| **i < 32));
    result.extend_front([4, 5, 6].iter().map(|i| i).filter(|i| **i < 32));
    assert_eq!(result.back(), &[1, 2, 3]);
    assert_eq!(result.front(), &[4, 5, 6])
}


#[test]
fn text_extend_vec() {
    let mut result = TwoSidedVec::<u32>::new();
    result.extend_back(vec![3, 2, 1]);
    result.extend_front(vec![4, 5, 6]);
    assert_eq!(result.back(), &[1, 2, 3]);
    assert_eq!(result.front(), &[4, 5, 6]);
}

#[test]
fn text_extend_trusted_len() {
    let mut result = TwoSidedVec::<u32>::new();
    result.extend_back(vec![3, 2, 1].into_iter().map(|i| i));
    result.extend_front(vec![4, 5, 6].into_iter().map(|i| i));
    assert_eq!(result.back(), &[1, 2, 3]);
    assert_eq!(result.front(), &[4, 5, 6])
}


#[test]
fn text_extend_default() {
    let mut result = TwoSidedVec::<u32>::new();
    result.extend_back(vec![3, 2, 1].into_iter().filter(|i| *i < 32));
    result.extend_front(vec![4, 5, 6].into_iter().filter(|i| *i < 32));
    assert_eq!(result.back(), &[1, 2, 3]);
    assert_eq!(result.front(), &[4, 5, 6])
}

#[test]
fn truncate_front() {
    let mut result = two_sided_vec![1, 2, 3; 4, 5, 6, 7];
    result.truncate_front(2);
    assert_eq!(result.front(), &[4, 5]);
    assert_eq!(result.back(), &[1, 2, 3]);
    let mut result = two_sided_vec![1, 2, 3; 4, 5, 6, 7];
    result.truncate_front(1);
    assert_eq!(result.front(), &[4]);
    assert_eq!(result.back(), &[1, 2, 3]);
    let mut result = two_sided_vec![1, 2, 3; 4, 5, 6, 7];
    result.truncate_front(0);
    assert_eq!(result.front(), &[]);
    assert_eq!(result.back(), &[1, 2, 3]);
    let mut result = two_sided_vec![1, 2, 3; 4, 5, 6, 7];
    result.truncate_front(20);
    assert_eq!(result.front(), &[4, 5, 6, 7]);
    assert_eq!(result.back(), &[1, 2, 3]);
}

#[test]
fn truncate_back() {
    let mut result = two_sided_vec![1, 2, 3; 4, 5, 6, 7];
    result.truncate_back(2);
    assert_eq!(result.front(), &[4, 5, 6, 7]);
    assert_eq!(result.back(), &[2, 3]);
    let mut result = two_sided_vec![1, 2, 3; 4, 5, 6, 7];
    result.truncate_back(1);
    assert_eq!(result.front(), &[4, 5, 6, 7]);
    assert_eq!(result.back(), &[3]);
    let mut result = two_sided_vec![2, 3; 4, 5, 6, 7];
    result.truncate_back(0);
    assert_eq!(result.front(), &[4, 5, 6, 7]);
    assert_eq!(result.back(), &[]);
    let mut result = two_sided_vec![1, 2, 3; 4, 5, 6, 7];
    result.truncate_back(20);
    assert_eq!(result.front(), &[4, 5, 6, 7]);
    assert_eq!(result.back(), &[1, 2, 3]);
}

fn assert_expected<T: Debug + Eq + Clone>(
    target: &TwoSidedVec<T>,
    mut expected_back: Vec<T>,
    expected_front: Vec<T>
) {
    expected_back.reverse();
    let expected_start = -(expected_back.len() as isize);
    let expected_end = expected_front.len() as isize;
    assert_eq!(target.start(), expected_start);
    assert_eq!(target.end(), expected_end);
    assert_eq!(target.len(), expected_back.len() + expected_front.len());
    assert_eq!(&target[..0], &*expected_back);
    assert_eq!(&target[0..], &*expected_front);
    for (index, expected) in expected_back.iter().rev().enumerate() {
        assert_eq!(&target[-(index as isize) - 1], expected);
    }
    for (index, expected) in expected_front.iter().enumerate() {
        assert_eq!(&target[(index as isize)], expected);
    }
    let entire_expected = expected_back.iter()
        .chain(expected_front.iter())
        .cloned()
        .collect::<Vec<T>>();
    assert_eq!(&target[..], &*entire_expected);
}
fn expected_back() -> Vec<i32> {
    (0..30).map(|i| i * -2).collect()
}
fn expected_front() -> Vec<i32> {
    (0..30).map(|i| i * 2).collect()
}