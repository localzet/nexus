//! Аутентификация и управление доступом

use std::collections::{HashMap, HashSet};
use anyhow::{Result, anyhow};

/// User role
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Role {
    Admin,
    DBA,
    User,
    Guest,
    Custom(String),
}

/// Database operation permission
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Permission {
    Select,
    Insert,
    Update,
    Delete,
    Create,
    Drop,
    Alter,
    Admin,
}

/// User account
#[derive(Debug, Clone)]
pub struct User {
    pub username: String,
    pub password_hash: String, // Should use bcrypt in production
    pub roles: HashSet<Role>,
    pub is_active: bool,
    pub created_at: String,
}

/// Role definition with permissions
#[derive(Debug, Clone)]
pub struct RoleDefinition {
    pub role: Role,
    pub permissions: HashSet<Permission>,
    pub table_permissions: HashMap<String, HashSet<Permission>>,
}

/// Authentication manager
pub struct AuthManager {
    users: HashMap<String, User>,
    roles: HashMap<Role, RoleDefinition>,
    current_user: Option<String>,
}

impl AuthManager {
    pub fn new() -> Self {
        let mut manager = Self {
            users: HashMap::new(),
            roles: HashMap::new(),
            current_user: None,
        };

        // Initialize default roles
        manager.init_default_roles();
        manager
    }

    fn init_default_roles(&mut self) {
        // Admin role - full permissions
        let mut admin_perms = HashSet::new();
        admin_perms.insert(Permission::Select);
        admin_perms.insert(Permission::Insert);
        admin_perms.insert(Permission::Update);
        admin_perms.insert(Permission::Delete);
        admin_perms.insert(Permission::Create);
        admin_perms.insert(Permission::Drop);
        admin_perms.insert(Permission::Alter);
        admin_perms.insert(Permission::Admin);

        self.roles.insert(
            Role::Admin,
            RoleDefinition {
                role: Role::Admin,
                permissions: admin_perms,
                table_permissions: HashMap::new(),
            },
        );

        // User role - basic read/write
        let mut user_perms = HashSet::new();
        user_perms.insert(Permission::Select);
        user_perms.insert(Permission::Insert);
        user_perms.insert(Permission::Update);
        user_perms.insert(Permission::Delete);

        self.roles.insert(
            Role::User,
            RoleDefinition {
                role: Role::User,
                permissions: user_perms,
                table_permissions: HashMap::new(),
            },
        );

        // Guest role - read-only
        let mut guest_perms = HashSet::new();
        guest_perms.insert(Permission::Select);

        self.roles.insert(
            Role::Guest,
            RoleDefinition {
                role: Role::Guest,
                permissions: guest_perms,
                table_permissions: HashMap::new(),
            },
        );
    }

    /// Create a new user
    pub fn create_user(&mut self, username: String, password: String, role: Role) -> Result<()> {
        if self.users.contains_key(&username) {
            return Err(anyhow!("User '{}' already exists", username));
        }

        // In production, use bcrypt: bcrypt::hash(&password, 12)
        let password_hash = format!("hash_{}", password);

        let mut roles = HashSet::new();
        roles.insert(role);

        let user = User {
            username,
            password_hash,
            roles,
            is_active: true,
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        self.users.insert(user.username.clone(), user);
        Ok(())
    }

    /// Authenticate user
    pub fn authenticate(&mut self, username: &str, password: &str) -> Result<()> {
        let user = self.users.get(username)
            .ok_or_else(|| anyhow!("User '{}' not found", username))?;

        if !user.is_active {
            return Err(anyhow!("User '{}' is inactive", username));
        }

        // Simple check - in production use bcrypt::verify
        let password_hash = format!("hash_{}", password);
        if user.password_hash != password_hash {
            return Err(anyhow!("Invalid password"));
        }

        self.current_user = Some(username.to_string());
        Ok(())
    }

    /// Check if current user has permission
    pub fn has_permission(&self, permission: Permission) -> Result<bool> {
        let username = self.current_user.as_ref()
            .ok_or_else(|| anyhow!("No user logged in"))?;

        let user = self.users.get(username)
            .ok_or_else(|| anyhow!("User not found"))?;

        for role in &user.roles {
            if let Some(role_def) = self.roles.get(role) {
                if role_def.permissions.contains(&permission) {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Check table-level permission
    pub fn has_table_permission(&self, table: &str, permission: Permission) -> Result<bool> {
        let username = self.current_user.as_ref()
            .ok_or_else(|| anyhow!("No user logged in"))?;

        let user = self.users.get(username)
            .ok_or_else(|| anyhow!("User not found"))?;

        // First check sys permission
        if !self.has_permission(permission)? {
            return Ok(false);
        }

        // Then check table-specific permission
        for role in &user.roles {
            if let Some(role_def) = self.roles.get(role) {
                if let Some(table_perms) = role_def.table_permissions.get(table) {
                    if table_perms.contains(&permission) {
                        return Ok(true);
                    }
                }
            }
        }

        // Fall back to system-wide permission
        Ok(true)
    }

    /// Grant permission to role
    pub fn grant_permission_to_role(&mut self, role: Role, permission: Permission) -> Result<()> {
        let role_def = self.roles.get_mut(&role)
            .ok_or_else(|| anyhow!("Role not found"))?;

        role_def.permissions.insert(permission);
        Ok(())
    }

    /// Revoke permission from role
    pub fn revoke_permission_from_role(&mut self, role: Role, permission: Permission) -> Result<()> {
        let role_def = self.roles.get_mut(&role)
            .ok_or_else(|| anyhow!("Role not found"))?;

        role_def.permissions.remove(&permission);
        Ok(())
    }

    /// Get current user
    pub fn current_user(&self) -> Option<&str> {
        self.current_user.as_deref()
    }

    /// Logout current user
    pub fn logout(&mut self) {
        self.current_user = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_user() {
        let mut auth = AuthManager::new();
        assert!(auth.create_user("alice".to_string(), "password123".to_string(), Role::User).is_ok());
        assert!(auth.users.contains_key("alice"));
    }

    #[test]
    fn test_duplicate_user() {
        let mut auth = AuthManager::new();
        auth.create_user("alice".to_string(), "password123".to_string(), Role::User).unwrap();
        assert!(auth.create_user("alice".to_string(), "password456".to_string(), Role::User).is_err());
    }

    #[test]
    fn test_authenticate_user() {
        let mut auth = AuthManager::new();
        auth.create_user("alice".to_string(), "password123".to_string(), Role::User).unwrap();
        assert!(auth.authenticate("alice", "password123").is_ok());
        assert_eq!(auth.current_user(), Some("alice"));
    }

    #[test]
    fn test_invalid_password() {
        let mut auth = AuthManager::new();
        auth.create_user("alice".to_string(), "password123".to_string(), Role::User).unwrap();
        assert!(auth.authenticate("alice", "wrongpassword").is_err());
    }

    #[test]
    fn test_permission_check() {
        let mut auth = AuthManager::new();
        auth.create_user("alice".to_string(), "password123".to_string(), Role::User).unwrap();
        auth.authenticate("alice", "password123").unwrap();
        
        // User role should have SELECT permission
        assert!(auth.has_permission(Permission::Select).unwrap());
        // But not ADMIN permission
        assert!(!auth.has_permission(Permission::Admin).unwrap());
    }

    #[test]
    fn test_admin_permissions() {
        let mut auth = AuthManager::new();
        auth.create_user("admin".to_string(), "admin123".to_string(), Role::Admin).unwrap();
        auth.authenticate("admin", "admin123").unwrap();
        
        // Admin has all permissions
        assert!(auth.has_permission(Permission::Select).unwrap());
        assert!(auth.has_permission(Permission::Admin).unwrap());
        assert!(auth.has_permission(Permission::Drop).unwrap());
    }

    #[test]
    fn test_logout() {
        let mut auth = AuthManager::new();
        auth.create_user("alice".to_string(), "password123".to_string(), Role::User).unwrap();
        auth.authenticate("alice", "password123").unwrap();
        assert_eq!(auth.current_user(), Some("alice"));
        
        auth.logout();
        assert_eq!(auth.current_user(), None);
    }

    #[test]
    fn test_grant_permission() {
        let mut auth = AuthManager::new();
        auth.grant_permission_to_role(Role::Guest, Permission::Insert).unwrap();
        
        auth.create_user("guest".to_string(), "guest123".to_string(), Role::Guest).unwrap();
        auth.authenticate("guest", "guest123").unwrap();
        
        // Guest now has INSERT permission
        assert!(auth.has_permission(Permission::Insert).unwrap());
    }
}
