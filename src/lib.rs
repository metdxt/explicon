use std::{env::var, str::FromStr};

use serde::{Deserialize, Serialize};

/// Represents errors that can occur during configuration value resolution.
#[derive(thiserror::Error, Debug)]
pub enum ExpliconError {
    /// Occurs when an environment variable can't be resolved.
    #[error("Error while resolving env var: {0}")]
    Var(#[from] std::env::VarError),
    /// Generic error container for other resolution failures.
    #[error("{0}")]
    Other(String),
}

/// Result type alias using [`ExpliconError`] for error handling in configuration resolution.
pub type Result<T> = std::result::Result<T, ExpliconError>;

/// A configuration value that can be sourced either directly or from an environment variable.
///
/// Supports deserialization from both formats:
/// - Direct value representation (e.g., `42` or `"direct_value"`)
/// - Environment variable reference (e.g., `{ "env": "VAR_NAME" }`)
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Sourced<T> {
    /// Value should be read from the specified environment variable
    Env(String),

    /// Directly provided value that doesn't require resolution
    #[serde(untagged)]
    Value(T),
}


impl<T> Sourced<T>
where
    T: FromStr,
    T: Clone,
    <T as FromStr>::Err: ToString,
{
    /// Resolves the configuration value by parsing if sourced from an environment variable.
    ///
    /// Use this method when the target type `T` implements [`FromStr`] (e.g., numbers, booleans).
    ///
    /// # Returns
    /// - `Ok(T)` with direct value if using [`Sourced::Value`]
    /// - `Ok(T)` with **parsed** environment variable value if using [`Sourced::Env`]
    ///
    /// # Errors
    /// - [`ExpliconError::Var`] if environment variable lookup fails
    /// - [`ExpliconError::Other`] if environment variable value parsing fails (via [`FromStr`])
    pub fn resolve(&self) -> Result<T> {
        match self {
            Self::Value(value) => Ok(value.clone()),
            Self::Env(var_name) => {
                let var_value = var(var_name)?; // Получаем String
                // Используем parse, т.к. T: FromStr
                let value = var_value
                    .parse::<T>()
                    .map_err(|e| ExpliconError::Other(e.to_string()))?;
                Ok(value)
            }
        }
    }

     /// Resolves the value using `resolve()` or returns type's default if resolution fails.
    pub fn resolve_or_default(&self) -> Result<T>
    where
        T: Default,
    {
        self.resolve().or_else(|_| Ok(T::default()))
    }

    /// Resolves the value using `resolve()` or returns the provided fallback value if resolution fails.
    pub fn resolve_or(&self, fallback: T) -> T {
        self.resolve().unwrap_or(fallback)
    }

    /// Resolves the value using `resolve()` and validates it against a predicate.
    pub fn resolve_and_validate<F>(&self, validator: F) -> Result<T>
    where
        F: FnOnce(&T) -> bool,
    {
        let value = self.resolve()?;
        if validator(&value) {
            Ok(value)
        } else {
            Err(ExpliconError::Other("Validation failed".into()))
        }
    }
}

impl<T> Sourced<T>
where
    T: From<String>,
    T: Clone,
{
    /// Resolves the configuration value by converting directly from the environment variable string.
    ///
    /// Use this method when the target type `T` implements [`From<String>`] but not necessarily [`FromStr`]
    /// (e.g., [`secrecy::SecretString`]).
    ///
    /// # Returns
    /// - `Ok(T)` with direct value if using [`Sourced::Value`]
    /// - `Ok(T)` with value created **directly via `From<String>`** from the environment variable if using [`Sourced::Env`]
    ///
    /// # Errors
    /// - [`ExpliconError::Var`] if environment variable lookup fails. Conversion via `From<String>` is assumed infallible.
    pub fn resolve_from_string(&self) -> Result<T> {
        match self {
            Self::Value(value) => Ok(value.clone()),
            Self::Env(var_name) => {
                let var_value = var(var_name)?; // Получаем String
                // Используем From<String>, т.к. T: From<String>
                Ok(T::from(var_value))
            }
        }
    }

     /// Resolves the value using `resolve_from_string()` or returns the provided fallback value.
    /// Note: Does not return default, as `From<String>` types might not have a meaningful default.
    pub fn resolve_from_string_or(&self, fallback: T) -> T {
        self.resolve_from_string().unwrap_or(fallback)
    }

     /// Resolves the value using `resolve_from_string()` and validates it against a predicate.
     pub fn resolve_from_string_and_validate<F>(&self, validator: F) -> Result<T>
    where
        F: FnOnce(&T) -> bool,
    {
        let value = self.resolve_from_string()?;
        if validator(&value) {
            Ok(value)
        } else {
            Err(ExpliconError::Other("Validation failed".into()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_value() {
        let sourced = Sourced::Value(42);
        assert_eq!(sourced.resolve().unwrap(), 42);
    }

    #[test]
    fn resolve_env_success() {
        let var_name = "TEST_RESOLVE_ENV_SUCCESS";
        let expected_value = 123;
        unsafe { std::env::set_var(var_name, expected_value.to_string()) };
        let sourced = Sourced::<i32>::Env(var_name.to_string());
        let result = sourced.resolve().unwrap();
        assert_eq!(result, expected_value);
        unsafe { std::env::remove_var(var_name) };
    }

    #[test]
    fn resolve_env_var_not_found() {
        let var_name = "NON_EXISTENT_VAR_XYZ123";
        unsafe { std::env::remove_var(var_name) };
        let sourced = Sourced::<i32>::Env(var_name.to_string());
        let result = sourced.resolve();
        assert!(matches!(result, Err(ExpliconError::Var(_))));
    }

    #[test]
    fn resolve_env_var_invalid_parse() {
        let var_name = "TEST_INVALID_PARSE";
        unsafe { std::env::set_var(var_name, "abc") };
        let sourced = Sourced::<i32>::Env(var_name.to_string());
        let result = sourced.resolve();
        assert!(matches!(result, Err(ExpliconError::Other(_))));
        unsafe { std::env::remove_var(var_name) };
    }

    #[test]
    fn resolve_or_default_env_missing() {
        let var_name = "NON_EXISTENT_VAR_FOR_DEFAULT";
        unsafe { std::env::remove_var(var_name) };
        let sourced = Sourced::<i32>::Env(var_name.to_string());
        let result = sourced.resolve_or_default().unwrap();
        assert_eq!(result, i32::default());
    }

    #[test]
    fn resolve_or_default_parse_error() {
        let var_name = "TEST_PARSE_ERROR_DEFAULT";
        unsafe { std::env::set_var(var_name, "abc") };
        let sourced = Sourced::<i32>::Env(var_name.to_string());
        let result = sourced.resolve_or_default().unwrap();
        assert_eq!(result, i32::default());
        unsafe { std::env::remove_var(var_name) };
    }

    #[test]
    fn resolve_or_default_success() {
        let var_name = "TEST_RESOLVE_OR_DEFAULT_SUCCESS";
        unsafe { std::env::set_var(var_name, "5") };
        let sourced = Sourced::<i32>::Env(var_name.to_string());
        let result = sourced.resolve_or_default().unwrap();
        assert_eq!(result, 5);
        unsafe { std::env::remove_var(var_name) };
    }

    #[test]
    fn resolve_and_validate_success() {
        let sourced = Sourced::Value(5);
        let result = sourced.resolve_and_validate(|v| *v == 5).unwrap();
        assert_eq!(result, 5);
    }

    #[test]
    fn resolve_and_validate_failure() {
        let sourced = Sourced::Value(5);
        let result = sourced.resolve_and_validate(|v| *v == 10);
        assert!(matches!(result, Err(ExpliconError::Other(_))));
    }

    #[test]
    fn resolve_and_validate_env_missing() {
        let var_name = "NON_EXISTENT_VAR_FOR_VALIDATE";
        unsafe { std::env::remove_var(var_name) };
        let sourced = Sourced::<i32>::Env(var_name.to_string());
        let result = sourced.resolve_and_validate(|_| true);
        assert!(matches!(result, Err(ExpliconError::Var(_))));
    }

    #[test]
    fn resolve_and_validate_env_invalid() {
        let var_name = "TEST_VALIDATE_ENV_INVALID";
        unsafe { std::env::set_var(var_name, "10") };
        let sourced = Sourced::<i32>::Env(var_name.to_string());
        let result = sourced.resolve_and_validate(|v| *v == 5);
        assert!(matches!(result, Err(ExpliconError::Other(_))));
        unsafe { std::env::remove_var(var_name) };
    }

    #[test]
    fn resolve_env_string() {
        let var_name = "TEST_ENV_STRING";
        let expected = "hello";
        unsafe { std::env::set_var(var_name, expected) };
        let sourced = Sourced::<String>::Env(var_name.to_string());
        let result = sourced.resolve().unwrap();
        assert_eq!(result, expected);
        unsafe { std::env::remove_var(var_name) };
    }

    #[test]
    fn resolve_env_bool() {
        let var_name = "TEST_ENV_BOOL";
        unsafe { std::env::set_var(var_name, "true") };
        let sourced = Sourced::<bool>::Env(var_name.to_string());
        let result = sourced.resolve().unwrap();
        assert!(result);
        unsafe { std::env::remove_var(var_name) };
    }
}
