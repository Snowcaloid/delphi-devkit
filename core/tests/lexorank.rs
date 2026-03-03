use ddk_core::lexorank::{Bucket, LexoRank, ParseError, Rank};

// ═══════════════════════════════════════════════════════════════════════════════
//  Rank – construction / validation
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn rank_valid_single_char() {
    assert!(Rank::new("h").is_ok());
}

#[test]
fn rank_valid_multi_char() {
    assert!(Rank::new("a1b").is_ok());
}

#[test]
fn rank_valid_digits_and_letters() {
    assert!(Rank::new("9z").is_ok());
}

#[test]
fn rank_rejects_empty_string() {
    assert!(Rank::new("").is_err());
}

#[test]
fn rank_rejects_trailing_zero() {
    assert!(Rank::new("h0").is_err());
}

#[test]
fn rank_rejects_uppercase() {
    assert!(Rank::new("A").is_err());
}

#[test]
fn rank_rejects_special_chars() {
    assert!(Rank::new("hello!").is_err());
}

#[test]
fn rank_try_from_str() {
    let rank: Result<Rank, ParseError> = "h".try_into();
    assert!(rank.is_ok());
    assert_eq!(rank.unwrap().value(), "h");
}

// ═══════════════════════════════════════════════════════════════════════════════
//  Rank – next / prev
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn rank_next_increments_last_char() {
    let rank = Rank::new("h").unwrap();
    assert_eq!(rank.next().value(), "i");
}

#[test]
fn rank_next_wraps_9_to_a() {
    let rank = Rank::new("9").unwrap();
    assert_eq!(rank.next().value(), "a");
}

#[test]
fn rank_next_z_appends_one() {
    let rank = Rank::new("z").unwrap();
    assert_eq!(rank.next().value(), "z1");
}

#[test]
fn rank_next_all_z_appends_one() {
    let rank = Rank::new("zz").unwrap();
    assert_eq!(rank.next().value(), "zz1");
}

#[test]
fn rank_prev_decrements_last_char() {
    let rank = Rank::new("h").unwrap();
    assert_eq!(rank.prev().value(), "g");
}

#[test]
fn rank_prev_a_wraps_to_9() {
    let rank = Rank::new("a").unwrap();
    assert_eq!(rank.prev().value(), "9");
}

#[test]
fn rank_prev_one_prepends_zero() {
    let rank = Rank::new("1").unwrap();
    assert_eq!(rank.prev().value(), "01");
}

#[test]
fn rank_prev_truncates_trailing_one() {
    // "a1" → decrement: last char is '1', special path → truncate to "a"
    let rank = Rank::new("a1").unwrap();
    assert_eq!(rank.prev().value(), "a");
}

#[test]
fn rank_next_then_prev_roundtrip() {
    let rank = Rank::new("h").unwrap();
    let next = rank.next();
    // next is "i", prev of "i" should be "h"
    assert_eq!(next.prev().value(), "h");
}

// ═══════════════════════════════════════════════════════════════════════════════
//  Rank – between
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn rank_between_returns_none_for_equal() {
    let rank = Rank::new("h").unwrap();
    assert!(rank.between(&rank).is_none());
}

#[test]
fn rank_between_returns_value_between() {
    let a = Rank::new("a").unwrap();
    let c = Rank::new("c").unwrap();
    let mid = a.between(&c).unwrap();
    assert!(mid > a);
    assert!(mid < c);
}

#[test]
fn rank_between_reversed_args_still_works() {
    let a = Rank::new("a").unwrap();
    let c = Rank::new("c").unwrap();
    let mid = c.between(&a).unwrap();
    assert!(mid > a);
    assert!(mid < c);
}

#[test]
fn rank_between_adjacent_appends_suffix() {
    let a = Rank::new("a").unwrap();
    let b = Rank::new("b").unwrap();
    let mid = a.between(&b).unwrap();
    assert!(mid > a);
    assert!(mid < b);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  Rank – from_range
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn rank_from_range_produces_ordered_ranks() {
    let total = 5;
    let ranks: Vec<Rank> = (0..total).map(|i| Rank::from_range(i, total)).collect();
    for window in ranks.windows(2) {
        assert!(window[0] < window[1], "{:?} should be < {:?}", window[0], window[1]);
    }
}

#[test]
fn rank_from_range_single_item() {
    let rank = Rank::from_range(0, 1);
    assert!(!rank.value().is_empty());
}

// ═══════════════════════════════════════════════════════════════════════════════
//  Bucket
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bucket_valid_values() {
    assert!(Bucket::new(0).is_ok());
    assert!(Bucket::new(1).is_ok());
    assert!(Bucket::new(2).is_ok());
}

#[test]
fn bucket_rejects_value_above_2() {
    assert!(Bucket::new(3).is_err());
    assert!(Bucket::new(255).is_err());
}

#[test]
fn bucket_next_wraps_around() {
    assert_eq!(Bucket::new(0).unwrap().next().value(), 1);
    assert_eq!(Bucket::new(1).unwrap().next().value(), 2);
    assert_eq!(Bucket::new(2).unwrap().next().value(), 0);
}

#[test]
fn bucket_prev_wraps_around() {
    assert_eq!(Bucket::new(0).unwrap().prev().value(), 2);
    assert_eq!(Bucket::new(1).unwrap().prev().value(), 0);
    assert_eq!(Bucket::new(2).unwrap().prev().value(), 1);
}

#[test]
fn bucket_try_from_u8() {
    let b: Result<Bucket, _> = 1u8.try_into();
    assert!(b.is_ok());
    assert_eq!(b.unwrap().value(), 1);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  LexoRank – parsing
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn lexorank_parse_valid() {
    let lr = LexoRank::from_string("1|h").unwrap();
    assert_eq!(lr.bucket().value(), 1);
    assert_eq!(lr.rank().value(), "h");
}

#[test]
fn lexorank_parse_bucket_0() {
    let lr = LexoRank::from_string("0|abc").unwrap();
    assert_eq!(lr.bucket().value(), 0);
    assert_eq!(lr.rank().value(), "abc");
}

#[test]
fn lexorank_parse_bucket_2() {
    let lr = LexoRank::from_string("2|z9a").unwrap();
    assert_eq!(lr.bucket().value(), 2);
}

#[test]
fn lexorank_parse_rejects_bad_bucket() {
    assert!(LexoRank::from_string("3|h").is_err());
}

#[test]
fn lexorank_parse_rejects_empty_rank() {
    assert!(LexoRank::from_string("1|").is_err());
}

#[test]
fn lexorank_parse_rejects_no_pipe() {
    assert!(LexoRank::from_string("no-pipe").is_err());
}

#[test]
fn lexorank_from_string_or_default_on_invalid() {
    let lr = LexoRank::from_string_or_default("garbage");
    assert_eq!(lr, LexoRank::default());
}

#[test]
fn lexorank_try_from_str() {
    let lr: Result<LexoRank, _> = "1|h".try_into();
    assert!(lr.is_ok());
}

// ═══════════════════════════════════════════════════════════════════════════════
//  LexoRank – ordering
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn lexorank_ordering_by_bucket_first() {
    let a = LexoRank::from_string("0|z").unwrap();
    let b = LexoRank::from_string("1|a").unwrap();
    assert!(a < b);
}

#[test]
fn lexorank_ordering_by_rank_within_bucket() {
    let a = LexoRank::from_string("1|a").unwrap();
    let b = LexoRank::from_string("1|h").unwrap();
    assert!(a < b);
}

#[test]
fn lexorank_equal() {
    let a = LexoRank::from_string("1|h").unwrap();
    let b = LexoRank::from_string("1|h").unwrap();
    assert_eq!(a, b);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  LexoRank – next / prev / between
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn lexorank_next_preserves_bucket() {
    let lr = LexoRank::from_string("1|h").unwrap();
    let next = lr.next();
    assert_eq!(next.bucket().value(), 1);
    assert!(next > lr);
}

#[test]
fn lexorank_prev_preserves_bucket() {
    let lr = LexoRank::from_string("1|h").unwrap();
    let prev = lr.prev();
    assert_eq!(prev.bucket().value(), 1);
    assert!(prev < lr);
}

#[test]
fn lexorank_between_works() {
    let a = LexoRank::from_string("1|a").unwrap();
    let b = LexoRank::from_string("1|z").unwrap();
    let mid = a.between(&b).unwrap();
    assert!(mid > a);
    assert!(mid < b);
}

#[test]
fn lexorank_between_equal_returns_none() {
    let a = LexoRank::from_string("1|h").unwrap();
    assert!(a.between(&a).is_none());
}

// ═══════════════════════════════════════════════════════════════════════════════
//  LexoRank – apply (rebalance)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn lexorank_apply_rebalances_list() {
    use ddk_core::lexorank::HasLexoRank;

    struct Item(LexoRank);
    impl HasLexoRank for Item {
        fn get_lexorank(&self) -> &LexoRank { &self.0 }
        fn set_lexorank(&mut self, lr: LexoRank) { self.0 = lr; }
    }

    let mut items = vec![
        Item(LexoRank::default()),
        Item(LexoRank::default()),
        Item(LexoRank::default()),
    ];
    let mut refs: Vec<&mut dyn HasLexoRank> = items.iter_mut().map(|i| i as &mut dyn HasLexoRank).collect();
    LexoRank::apply(&mut refs);

    // After rebalance, all ranks should be strictly ordered.
    let ranks: Vec<&LexoRank> = items.iter().map(|i| &i.0).collect();
    for w in ranks.windows(2) {
        assert!(w[0] < w[1], "{} should be < {}", w[0], w[1]);
    }
}

#[test]
fn lexorank_apply_empty_list() {
    use ddk_core::lexorank::HasLexoRank;
    let mut items: Vec<&mut dyn HasLexoRank> = Vec::new();
    LexoRank::apply(&mut items); // should not panic
}

// ═══════════════════════════════════════════════════════════════════════════════
//  LexoRank – Display / serde round-trip
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn lexorank_display() {
    let lr = LexoRank::from_string("1|h").unwrap();
    assert_eq!(format!("{}", lr), "1|h");
}

#[test]
fn lexorank_serde_roundtrip() {
    let lr = LexoRank::from_string("2|abc").unwrap();
    let json = serde_json::to_string(&lr).unwrap();
    let deserialized: LexoRank = serde_json::from_str(&json).unwrap();
    assert_eq!(lr, deserialized);
}

#[test]
fn lexorank_default_is_bucket_1() {
    let lr = LexoRank::default();
    assert_eq!(lr.bucket().value(), 1);
    assert_eq!(lr.rank().value(), "h");
}
