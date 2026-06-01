use crate::error::AppError;

pub fn validate_username(username: &str) -> Result<(), AppError> {
    if username.len() < 3 || username.len() > 32 {
        return Err(AppError::BadRequest(
            "Username must be 3-32 characters".into(),
        ));
    }
    if !username
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(AppError::BadRequest(
            "Username can only contain alphanumeric characters and underscores".into(),
        ));
    }
    Ok(())
}

pub fn validate_email(email: &str) -> Result<(), AppError> {
    if email.len() < 5 || email.len() > 254 {
        return Err(AppError::BadRequest("Invalid email address".into()));
    }
    let at = email
        .find('@')
        .ok_or_else(|| AppError::BadRequest("Invalid email address".into()))?;
    let after_at = &email[at + 1..];
    if !after_at.contains('.') || after_at.starts_with('.') || after_at.ends_with('.') {
        return Err(AppError::BadRequest("Invalid email address".into()));
    }
    Ok(())
}

pub fn validate_password(password: &str) -> Result<(), AppError> {
    if password.len() < 8 || password.len() > 128 {
        return Err(AppError::BadRequest(
            "Password must be 8-128 characters".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn username_min_length_is_three() {
        assert!(validate_username("ab").is_err());
        assert!(validate_username("abc").is_ok());
    }

    #[test]
    fn username_max_length_is_thirty_two() {
        assert!(validate_username(&"a".repeat(32)).is_ok());
        assert!(validate_username(&"a".repeat(33)).is_err());
    }

    #[test]
    fn username_rejects_non_alphanumeric() {
        assert!(validate_username("a-b").is_err());
        assert!(validate_username("a.b").is_err());
        assert!(validate_username("a b").is_err());
        assert!(validate_username("hello!").is_err());
    }

    #[test]
    fn username_accepts_underscore_and_alphanumeric() {
        assert!(validate_username("user_123").is_ok());
        assert!(validate_username("ABC_xyz").is_ok());
    }

    #[test]
    fn email_rejects_missing_at() {
        assert!(validate_email("nodomain").is_err());
        assert!(validate_email("no-at-sign").is_err());
    }

    #[test]
    fn email_rejects_no_dot_after_at() {
        assert!(validate_email("user@nodot").is_err());
    }

    #[test]
    fn email_rejects_dot_at_edge_of_domain() {
        assert!(validate_email("user@.example.com").is_err());
        assert!(validate_email("user@example.").is_err());
    }

    #[test]
    fn email_accepts_valid_addresses() {
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email("a.b+c@x.co.uk").is_ok());
    }

    #[test]
    fn email_rejects_too_short() {
        assert!(validate_email("a@b").is_err());
    }

    #[test]
    fn password_min_length_is_eight() {
        assert!(validate_password("1234567").is_err());
        assert!(validate_password("12345678").is_ok());
    }

    #[test]
    fn password_max_length_is_one_twenty_eight() {
        assert!(validate_password(&"x".repeat(128)).is_ok());
        assert!(validate_password(&"x".repeat(129)).is_err());
    }
}
