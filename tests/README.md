**To test only tests at the crate root level:**

```
cargo test --test '*'
```

**Test only at the crate main binary level:**

```
cargo test --lib pq
```

**Test a specific binary by name:**

```
cargo test --bin tiny
```
