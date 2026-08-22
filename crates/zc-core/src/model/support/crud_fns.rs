use crate::model::support::DbBmc;
use crate::model::support::prep_fields::{
	prep_fields_for_create, prep_fields_for_create_uid_included, prep_fields_for_update,
};
use crate::model::{EntityAction, Id, ModelEvent, ModelManager, RelIds, Result, get_model_bus};
use modql::SqliteFromRow;
use modql::field::{HasSqliteFields, SqliteFields};
use modql::filter::ListOptions;
use rusqlite::types::{ToSql, ToSqlOutput, Value, ValueRef};

pub const DEFAULT_LIST_LIMIT: i64 = 12000;

pub async fn create<MC>(mm: &ModelManager, fields: SqliteFields) -> Result<Id>
where
	MC: DbBmc,
{
	create_inner::<MC>(mm, fields, true, RelIds::default()).await
}

pub async fn update_with_rel_ids<MC>(
	mm: &ModelManager,
	id: Id,
	mut fields: SqliteFields,
	rel_ids: RelIds,
) -> Result<usize>
where
	MC: DbBmc,
{
	prep_fields_for_update::<MC>(&mut fields);

	// -- Build sql
	let sql = format!("UPDATE {} SET {} WHERE id = ?", MC::table_ref(), fields.sql_setters(),);

	// -- Execute the command
	let mut values = fields_to_values(&fields)?;
	values.push(Value::from(id));
	let db = mm.db();

	let count = db.exec(&sql, rusqlite::params_from_iter(&values)).await?;

	// -- Publish Model Event
	get_model_bus().publish(ModelEvent::new(
		MC::ENTITY_TYPE,
		EntityAction::Updated,
		Some(id),
		rel_ids,
	));

	Ok(count)
}

pub async fn update<MC>(mm: &ModelManager, id: Id, fields: SqliteFields) -> Result<usize>
where
	MC: DbBmc,
{
	update_with_rel_ids::<MC>(mm, id, fields, RelIds::default()).await
}

#[allow(unused)]
pub async fn create_where_not_exists<MC>(
	mm: &ModelManager,
	mut fields: SqliteFields,
	not_exists_fields: SqliteFields,
	not_exists_extra_static: Option<&str>, // hack for now for the task_id IS NULL
) -> Result<Option<Id>>
where
	MC: DbBmc,
{
	prep_fields_for_create::<MC>(&mut fields);

	let table = MC::table_ref();
	let columns = fields.sql_columns();
	let placeholders = fields.sql_placeholders();
	let mut where_clause = not_exists_fields
		.fields()
		.iter()
		.map(|f| format!("\"{}\" = ?", f.iden)) // won't work with rel.col
		.collect::<Vec<_>>()
		.join(" AND ");

	if let Some(extra_static) = not_exists_extra_static {
		where_clause.push_str(&format!(" AND {extra_static}"));
	}

	let sql = format!(
		"
INSERT INTO {table} ({columns}) SELECT {placeholders}
WHERE NOT EXISTS (
		SELECT 1 FROM {table} where {where_clause}
)
RETURNING id",
	);

	// -- Execute the command
	let fields = fields.extended(not_exists_fields);
	let values = fields_to_values(&fields)?;
	let db = mm.db();

	let id: Option<Id> = db.exec_returning_as_optional(&sql, rusqlite::params_from_iter(&values)).await?;

	if let Some(id) = id {
		get_model_bus().publish(ModelEvent::new(
			MC::ENTITY_TYPE,
			EntityAction::Created,
			Some(id),
			RelIds::default(),
		));
	}

	Ok(id)
}

#[allow(unused)]
pub async fn create_with_rel_ids<MC>(mm: &ModelManager, fields: SqliteFields, rel_ids: RelIds) -> Result<Id>
where
	MC: DbBmc,
{
	create_inner::<MC>(mm, fields, true, rel_ids).await
}

async fn create_inner<MC>(
	mm: &ModelManager,
	mut fields: SqliteFields,
	generate_uuid: bool,
	rel_ids: RelIds,
) -> Result<Id>
where
	MC: DbBmc,
{
	if generate_uuid {
		prep_fields_for_create::<MC>(&mut fields);
	} else {
		prep_fields_for_create_uid_included(&mut fields);
	}

	let sql = format!(
		"INSERT INTO {} ({}) VALUES ({}) RETURNING id",
		MC::table_ref(),
		fields.sql_columns(),
		fields.sql_placeholders()
	);

	// -- Execute the command
	let values = fields_to_values(&fields)?;
	let db = mm.db();

	let id: Id = db.exec_returning_as(&sql, rusqlite::params_from_iter(&values)).await?;

	// -- Publish Model Event
	get_model_bus().publish(ModelEvent::new(
		MC::ENTITY_TYPE,
		EntityAction::Created,
		Some(id),
		rel_ids,
	));

	Ok(id)
}

pub async fn get<MC, E>(mm: &ModelManager, id: Id) -> Result<E>
where
	MC: DbBmc,
	E: SqliteFromRow + Unpin + Send,
	E: HasSqliteFields,
{
	// -- Select
	let sql = format!(
		"SELECT {} FROM {} WHERE id = ? LIMIT 1",
		//
		E::sqlite_columns_for_select(),
		MC::table_ref(),
	);

	// -- Exec query
	let db = mm.db();
	let entity: E = db
		.fetch_first(&sql, [id])
		.await?
		.ok_or_else(|| format!("Cannot get entity '{}'", MC::TABLE))?;

	Ok(entity)
}

#[allow(unused)]
pub async fn batch_create_with_rel_ids<MC>(
	mm: &ModelManager,
	mut items: Vec<SqliteFields>,
	rel_ids: RelIds,
) -> Result<Vec<Id>>
where
	MC: DbBmc,
{
	if items.is_empty() {
		return Ok(Vec::new());
	}

	// Prepare each row fields (adds uid/ctime/mtime and table-specific defaults)
	for fields in items.iter_mut() {
		prep_fields_for_create::<MC>(fields);
	}

	let res = mm
		.db()
		.exec_in_tx(|tx_db| {
			let mut ids: Vec<Id> = Vec::with_capacity(items.len());
			for fields in items {
				let sql = format!(
					"INSERT INTO {} ({}) VALUES ({}) RETURNING id",
					MC::table_ref(),
					fields.sql_columns(),
					fields.sql_placeholders()
				);

				let values = fields_to_values(&fields)?;
				let id: Id = tx_db.exec_returning_as(&sql, rusqlite::params_from_iter(&values))?;
				ids.push(id);
			}
			Ok(ids)
		})
		.await?;

	// -- Publish Model Event
	get_model_bus().publish(ModelEvent::new(MC::ENTITY_TYPE, EntityAction::Created, None, rel_ids));

	Ok(res)
}

/// Helper to convert a Vec<T> into Vec<SqliteFields> using sqlite_not_none_fields().
#[allow(unused)]
pub fn map_items_to_sqlite_fields<T>(items: Vec<T>) -> Vec<SqliteFields>
where
	T: HasSqliteFields,
{
	items.into_iter().map(|it| it.sqlite_not_none_fields()).collect()
}

#[allow(unused)]
pub async fn first<MC, E>(
	mm: &ModelManager,
	list_options: Option<ListOptions>,
	filter_fields: Option<SqliteFields>,
) -> Result<Option<E>>
where
	MC: DbBmc,
	E: SqliteFromRow + Unpin + Send,
	E: HasSqliteFields,
{
	let list_options = if let Some(list_options) = list_options {
		list_options.with_limit(1)
	} else {
		ListOptions::from_limit(1)
	};
	let entities = list::<MC, E>(mm, Some(list_options), filter_fields).await?;
	Ok(entities.into_iter().next())
}

pub async fn list<MC, E>(
	mm: &ModelManager,
	list_options: Option<ListOptions>,
	filter_fields: Option<SqliteFields>,
) -> Result<Vec<E>>
where
	MC: DbBmc,
	E: SqliteFromRow + Unpin + Send,
	E: HasSqliteFields,
{
	let list_options = list_options.unwrap_or_default();
	let limit = list_options.limit.unwrap_or(DEFAULT_LIST_LIMIT);
	let order_by = list_options
		.order_bys
		.map(|ob| ob.join_for_sql())
		.unwrap_or_else(|| "id".to_string());
	// TODO: add the offset

	// -- Select
	let (sql, values) = if let Some(filter_fields) = filter_fields.as_ref() {
		// NOTE: For now only support =
		let where_clause = filter_fields
			.fields()
			.iter()
			.map(|f| format!("\"{}\" = ?", f.iden)) // won't work with rel.col
			.collect::<Vec<_>>()
			.join(" AND ");

		let sql = format!(
			"SELECT {} FROM {} WHERE {} ORDER BY {order_by} LIMIT {limit} ",
			E::sql_columns(),
			MC::table_ref(),
			where_clause,
		);

		(sql, fields_to_values(filter_fields)?)
	} else {
		let sql = format!(
			"SELECT {} FROM {} ORDER BY {order_by} LIMIT {limit} ",
			E::sql_columns(),
			MC::table_ref()
		);
		(sql, Vec::new())
	};

	// -- Exec query
	let db = mm.db();
	let entities: Vec<E> = db.fetch_all(&sql, rusqlite::params_from_iter(&values)).await?;

	Ok(entities)
}

// region:    --- Support

fn to_sqlite_value(val: &dyn ToSql) -> Result<Value> {
	let output = val.to_sql()?;
	let value = match output {
		ToSqlOutput::Borrowed(vr) => match vr {
			ValueRef::Null => Value::Null,
			ValueRef::Integer(v) => Value::Integer(v),
			ValueRef::Real(v) => Value::Real(v),
			ValueRef::Text(v) => {
				let s = std::str::from_utf8(v).map_err(|e| format!("Invalid utf-8 string: {e}"))?;
				Value::Text(s.to_string())
			}
			ValueRef::Blob(v) => Value::Blob(v.to_vec()),
		},
		ToSqlOutput::Owned(v) => v,
		#[allow(unreachable_patterns)]
		_ => return Err("Unsupported ToSqlOutput type".into()),
	};
	Ok(value)
}

fn fields_to_values(fields: &SqliteFields) -> Result<Vec<Value>> {
	let dyn_values = fields.values_as_dyn_to_sql_vec();
	let mut values = Vec::with_capacity(dyn_values.len());
	for val in dyn_values {
		values.push(to_sqlite_value(val)?);
	}
	Ok(values)
}

// endregion: --- Support
