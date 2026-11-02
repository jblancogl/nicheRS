//! Projects: named groups of an organisation's studies (jobs), e.g. one client file of a
//! consultancy. Purely organisational: a project never changes how a study is computed, and is
//! not part of the job specification or its hash. Local mode has projects without organisation.

use rusqlite::{OptionalExtension, params};

use crate::AppError;
use crate::accounts::{Principal, Scope};
use crate::dto::Project;
use crate::store::{Store, db, now};

const MAX_NAME: usize = 120;

fn clean_name(name: &str) -> Result<String, AppError> {
    let n = name.trim();
    if n.is_empty() {
        return Err(AppError::Invalid("give the project a name".into()));
    }
    if n.chars().count() > MAX_NAME {
        return Err(AppError::Invalid(format!(
            "project names have at most {MAX_NAME} characters"
        )));
    }
    Ok(n.to_string())
}

fn org_of(scope: &Scope) -> Option<&str> {
    match scope {
        Scope::All => None,
        Scope::Org(o) => Some(o.as_str()),
    }
}

fn duplicate(e: rusqlite::Error, name: &str) -> AppError {
    if let rusqlite::Error::SqliteFailure(f, _) = &e
        && f.code == rusqlite::ErrorCode::ConstraintViolation
    {
        return AppError::Conflict(format!("a project named “{name}” already exists"));
    }
    db(e)
}

impl Store {
    /// Projects visible in `scope` (an organisation's own; local mode: those without
    /// organisation), with their number of studies, most recently active first.
    pub fn project_list(&self, scope: &Scope) -> Result<Vec<Project>, AppError> {
        let org = org_of(scope);
        self.with(|c| {
            let mut st = c
                .prepare(
                    "SELECT p.id, p.name, p.created_at, COUNT(j.id), MAX(j.created_at)
                     FROM projects p LEFT JOIN jobs j ON j.project_id = p.id
                     WHERE p.org_id IS ?1
                     GROUP BY p.id
                     ORDER BY ifnull(MAX(j.created_at), p.created_at) DESC",
                )
                .map_err(db)?;
            st.query_map([org], |r| {
                Ok(Project {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    created_at: r.get(2)?,
                    studies: r.get::<_, i64>(3)?.max(0) as u64,
                    last_study_at: r.get(4)?,
                })
            })
            .map_err(db)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db)
        })
    }

    /// The project if `scope` may see it; another organisation's project is "not found".
    pub fn project(&self, id: &str, scope: &Scope) -> Result<Project, AppError> {
        self.project_list(scope)?
            .into_iter()
            .find(|p| p.id == id)
            .ok_or_else(|| AppError::NotFound("project".into()))
    }

    pub fn project_create(
        &self,
        scope: &Scope,
        by: Option<&Principal>,
        name: &str,
    ) -> Result<Project, AppError> {
        let name = clean_name(name)?;
        let id = uuid::Uuid::now_v7().to_string();
        self.with(|c| {
            c.execute(
                "INSERT INTO projects(id, org_id, name, created_at, created_by) VALUES(?1,?2,?3,?4,?5)",
                params![id, org_of(scope), name, now(), by.map(|p| p.user_id.as_str())],
            )
            .map_err(|e| duplicate(e, &name))
            .map(|_| ())
        })?;
        self.project(&id, scope)
    }

    pub fn project_rename(&self, id: &str, scope: &Scope, name: &str) -> Result<Project, AppError> {
        self.project(id, scope)?;
        let name = clean_name(name)?;
        self.with(|c| {
            c.execute("UPDATE projects SET name=?2 WHERE id=?1", params![id, name])
                .map_err(|e| duplicate(e, &name))
                .map(|_| ())
        })?;
        self.project(id, scope)
    }

    /// Delete a project; its studies are kept, without project.
    pub fn project_delete(&self, id: &str, scope: &Scope) -> Result<(), AppError> {
        self.project(id, scope)?;
        self.with(|c| {
            let tx = c.unchecked_transaction().map_err(db)?;
            tx.execute("UPDATE jobs SET project_id=NULL WHERE project_id=?1", [id])
                .map_err(db)?;
            tx.execute("DELETE FROM projects WHERE id=?1", [id])
                .map_err(db)?;
            tx.commit().map_err(db)
        })
    }

    /// Rename a study and/or move it to a project of the same organisation (`Some(None)`: no
    /// project). The caller has already checked the study belongs to `scope`.
    pub fn job_update(
        &self,
        id: &str,
        scope: &Scope,
        name: Option<&str>,
        project: Option<Option<&str>>,
    ) -> Result<(), AppError> {
        if let Some(Some(p)) = project {
            self.project(p, scope)?;
        }
        let name = name
            .map(|n| {
                let n = n.trim();
                if n.is_empty() {
                    Err(AppError::Invalid("give the study a name".into()))
                } else if n.chars().count() > 200 {
                    Err(AppError::Invalid(
                        "study names have at most 200 characters".into(),
                    ))
                } else {
                    Ok(n.to_string())
                }
            })
            .transpose()?;
        self.with(|c| {
            if let Some(n) = &name {
                c.execute("UPDATE jobs SET name=?2 WHERE id=?1", params![id, n])
                    .map_err(db)?;
            }
            if let Some(p) = project {
                c.execute("UPDATE jobs SET project_id=?2 WHERE id=?1", params![id, p])
                    .map_err(db)?;
            }
            Ok(())
        })
    }

    /// Project of a study (`None`: no such study; `Some(None)`: no project).
    pub fn job_project(&self, id: &str) -> Result<Option<Option<String>>, AppError> {
        self.with(|c| {
            c.query_row("SELECT project_id FROM jobs WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .optional()
            .map_err(db)
        })
    }
}
