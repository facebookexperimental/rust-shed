/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! `mysql_client`'s [Query] type and [query] macro.
//!
//! These are factored into a separate crate so that other crates can implement
//! the trait without having to pull in all the mysql_client dependencies.

mod to_sql;
pub use to_sql::ToSQL;
pub use to_sql::TryToSQL;

mod row_field;
pub use row_field::OptionalTryFromRowField;
pub use row_field::RowField;
pub use row_field::TryFromRowField;
pub use row_field::ValueError;
pub use row_field::opt_try_from_rowfield;

/**
Unpack result for `field` from query results contained in `row` into [`T`]

See [`mysql_client`] module documentation for examples.

# Arguments
## `$row`
[`RowFieldIterator`] from within `fn try_from` in `impl TryFrom<RowFieldIterator> for $some_struct`

## `$some_struct`
This has no affect on the behavior of the macro, but it is required
so informative error messages can be created.
For example, if `column` does not exist in `row`, it will not be known until runtime. Same goes for a `column` which does
exist but the type does not unpack into the struct field's type.

The original `MysqlError` does not help you by saying what field on your struct you were attempting to unpack a result.

## `$field`
The name of a column from the query results in `row`. This column should be one of the `SELECT` expressions in
the Query whose results are held by `row`.

[`row_field_macros`]: mysql_client::row_field_macros
[`RowFieldIterator`]: mysql_client::RowFieldIterator
*/
#[macro_export]
macro_rules! sql_field {
    ($row:expr, $some_struct:ident, $field:expr) => {{
        $crate::TryFromRowField::try_from($row.get_by_field_name($field).map_err(|e| {
            ::mysql_client::MysqlError::SchemaError(format!(
                "Could not find column '{}' on struct '{}'. Perhaps a typo? Original Error: {}",
                $field,
                ::std::any::type_name::<$some_struct>(),
                e.to_string(),
            ))
        })?)
        .map_err(|e| {
            ::mysql_client::MysqlError::SchemaError(format!(
                "{}.{} wrong type: {}",
                ::std::any::type_name::<$some_struct>(),
                $field,
                e.to_string()
            ))
        })?
    }};
}

/**
Unpack result for `field` from query results contained in `row` into `Option<T>`

See [`mysql_client`] module documentation for examples.

# Arguments
## `$row`
[`RowFieldIterator`] from within `fn try_from` in `impl TryFrom<RowFieldIterator> for $some_struct`

## `$some_struct`
This has no affect on the behavior of the macro, but it is required
so informative error messages can be created.
For example, if `column` does not exist in `row`, it will not be known until runtime. Same goes for a `column` which does
exist but the type does not unpack into the struct field's type.

The original `MysqlError` does not help you by saying what field on your struct you were attempting to unpack a result.

## `$field`
The name of a column from the query results in `row`. This column should be one of the `SELECT` expressions in
the Query whose results are held by `row`.

[`row_field_macros`]: mysql_client::row_field_macros
[`RowFieldIterator`]: mysql_client::RowFieldIterator
*/
#[macro_export]
macro_rules! option_sql_field {
    ($row:expr, $some_struct:ident, $field:expr) => {{
        $crate::OptionalTryFromRowField::try_from_opt($row.get_by_field_name($field).map_err(
            |e| {
                ::mysql_client::MysqlError::SchemaError(format!(
                    "Could not find column '{}' on struct '{}'. Perhaps a typo? Original Error: {}",
                    $field,
                    ::std::any::type_name::<$some_struct>(),
                    e.to_string(),
                ))
            },
        )?)
        .map_err(|e| {
            ::mysql_client::MysqlError::SchemaError(format!(
                "{}.{} wrong type: {}",
                ::std::any::type_name::<$some_struct>(),
                $field,
                e.to_string()
            ))
        })?
    }};
}
