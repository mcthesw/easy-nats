#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectPattern {
    tokens: Vec<SubjectToken>,
}

impl SubjectPattern {
    pub fn parse(pattern: &str) -> Result<Self, String> {
        let trimmed = pattern.trim();
        if trimmed.is_empty() {
            return Err("Subject pattern cannot be empty".to_string());
        }
        let mut tokens = Vec::new();
        for (idx, token) in trimmed.split('.').enumerate() {
            if token.is_empty() {
                return Err("Subject pattern cannot contain empty tokens".to_string());
            }
            let parsed = match token {
                "*" => SubjectToken::One,
                ">" => {
                    if idx != trimmed.split('.').count() - 1 {
                        return Err("The > wildcard must be the final token".to_string());
                    }
                    SubjectToken::Tail
                }
                literal if literal.contains('*') || literal.contains('>') => {
                    return Err("Wildcards must occupy the whole token".to_string());
                }
                literal => SubjectToken::Literal(literal.to_string()),
            };
            tokens.push(parsed);
        }
        Ok(Self { tokens })
    }

    pub fn matches(&self, subject: &str) -> bool {
        let subject_tokens: Vec<&str> = subject.trim().split('.').collect();
        if subject.trim().is_empty() || subject_tokens.iter().any(|token| token.is_empty()) {
            return false;
        }
        let mut subject_idx = 0;
        for (pattern_idx, token) in self.tokens.iter().enumerate() {
            match token {
                SubjectToken::Literal(literal) => {
                    if subject_tokens.get(subject_idx).copied() != Some(literal.as_str()) {
                        return false;
                    }
                    subject_idx += 1;
                }
                SubjectToken::One => {
                    if subject_tokens.get(subject_idx).is_none() {
                        return false;
                    }
                    subject_idx += 1;
                }
                SubjectToken::Tail => {
                    return pattern_idx == self.tokens.len() - 1
                        && subject_tokens.len() > subject_idx;
                }
            }
        }
        subject_idx == subject_tokens.len()
    }

    pub fn specificity(&self) -> u32 {
        self.tokens
            .iter()
            .map(|token| match token {
                SubjectToken::Literal(_) => 10,
                SubjectToken::One => 3,
                SubjectToken::Tail => 1,
            })
            .sum::<u32>()
            + self.tokens.len() as u32
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SubjectToken {
    Literal(String),
    One,
    Tail,
}
