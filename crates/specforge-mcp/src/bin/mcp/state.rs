//! Global state and rate limiting for the MCP server
//!
//! Contains rate limiters and concurrency controls.

use once_cell::sync::Lazy;
use tokio::sync::Semaphore;
use specforge_lib::utils::shared_store::RateLimiter;
use super::security::ToolCategory;

// ============================================================================
// Rate Limiting
// ============================================================================

/// Global rate limiter (100 requests/minute) - applies to all requests
pub static RATE_LIMITER: Lazy<RateLimiter> = Lazy::new(|| RateLimiter::default());

/// Tool-level rate limiters with category-specific limits
pub struct ToolRateLimiters {
    /// Read-only tools: 200 requests/minute (generous)
    read_only: RateLimiter,
    /// Write tools: 30 requests/minute (moderate)
    write: RateLimiter,
    /// Execute tools: 10 requests/minute (strict)
    execute: RateLimiter,
}

impl Default for ToolRateLimiters {
    fn default() -> Self {
        Self {
            read_only: RateLimiter::new(200, 60),   // 200/min
            write: RateLimiter::new(30, 60),        // 30/min
            execute: RateLimiter::new(10, 60),      // 10/min
        }
    }
}

impl ToolRateLimiters {
    /// Check rate limit based on tool category
    pub fn check(&self, category: ToolCategory) -> Result<(), String> {
        match category {
            ToolCategory::ReadOnly => self.read_only.check_and_increment(),
            ToolCategory::Write => self.write.check_and_increment(),
            ToolCategory::Execute => self.execute.check_and_increment(),
        }
    }

    /// Get the limit description for error messages
    pub fn get_limit_description(&self, category: ToolCategory) -> &'static str {
        match category {
            ToolCategory::ReadOnly => "200 requests/minute for read-only tools",
            ToolCategory::Write => "30 requests/minute for write tools",
            ToolCategory::Execute => "10 requests/minute for execute tools",
        }
    }
}

pub static TOOL_RATE_LIMITERS: Lazy<ToolRateLimiters> = Lazy::new(ToolRateLimiters::default);

// ============================================================================
// Concurrency Control
// ============================================================================

/// Maximum concurrent action executions (scripts, webhooks, workflows)
pub const MAX_CONCURRENT_ACTIONS: usize = 10;

/// Global semaphore for action concurrency control
pub static ACTION_SEMAPHORE: Lazy<Semaphore> = Lazy::new(|| Semaphore::new(MAX_CONCURRENT_ACTIONS));
