use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use wither::bson::{doc, oid::ObjectId};
use wither::mongodb::Client as Mongo;
use wither::prelude::Model;

pub async fn get_all_guilds(client: &Mongo) -> Result<Vec<Guild>, &str> {
	let db = client.database("bowot");
	if let Ok(mut cursor) = Guild::find(&db, None, None).await {
		let mut guilds: Vec<Guild> = Vec::new();
		while let Some(res) = cursor.next().await {
			if let Ok(guild) = res {
				guilds.push(guild);
			}
		}
		return Ok(guilds);
	}
	Err("Db not found")
}

#[derive(Debug, Model, Serialize, Deserialize)]
#[model(index(keys = r#"doc!{"guild_id": 1}"#, options = r#"doc!{"unique": true}"#))]
pub struct Guild {
	#[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
	pub id: Option<ObjectId>,
	pub guild_id: i64,
	pub self_roles: Vec<i64>,
	pub members: Vec<Member>,
	pub prefix: String,
	pub default_role: i64,
	pub custom_commands: Vec<CustomCommand>,
	pub trigger_phrases: Vec<TriggerPhrase>,
	pub disabled_commands: Vec<String>,
	pub hydrate: Vec<i64>,
}

impl Guild {
	pub fn new(guild_id: i64, prefix: String) -> Self {
		Guild {
			id: None,
			guild_id: guild_id,
			self_roles: vec![],
			members: vec![],
			prefix: prefix,
			default_role: 0,
			custom_commands: vec![],
			trigger_phrases: vec![TriggerPhrase {
				phrase: "hi".to_string(),
				reply: "hi".to_string(),
				emote: '👋',
			}],
			disabled_commands: vec![],
			hydrate: vec![],
		}
	}

	pub async fn from_db(client: &Mongo, guild_id: i64) -> Result<Self, &'static str> {
		let db = client.database("bowot");
		if let Ok(_g) = Guild::find_one(&db, doc! {"guild_id": guild_id}, None).await {
			if let Some(g) = _g {
				return Ok(g);
			}
		}
		Err("Db not found")
	}

	pub async fn save_guild(&mut self, client: &Mongo) -> Result<&mut Self, &str> {
		let db = client.database("bowot");
		if let Ok(_) = self.save(&db, None).await {
			return Ok(self);
		};
		Err("Db not found")
	}

	pub async fn delete_guild(&mut self, client: &Mongo) -> Result<(), &str> {
		let db = client.database("bowot");
		if let Ok(_) = self.delete(&db).await {
			return Ok(());
		};
		Err("Db not found")
	}

	pub fn add_member(&mut self, member_id: i64) -> Result<&mut Self, &str> {
		if self.members.iter().any(|y| y.id == member_id) {
			return Err("Member already exists");
		}
		self.members.push(Member::new(member_id));
		Ok(self)
	}

	pub fn remove_member(&mut self, member_id: i64) -> Result<&mut Self, &str> {
		for (i, y) in self.members.iter().enumerate() {
			if y.id == member_id {
				self.members.remove(i);
				return Ok(self);
			}
		}
		Err("Member doesn't exist")
	}

	pub fn add_self_role(&mut self, role_id: i64) -> Result<&mut Self, &str> {
		match self.self_roles.binary_search(&role_id) {
			Ok(_) => Err("Self role already exists"),
			Err(_) => {
				self.self_roles.push(role_id);
				Ok(self)
			}
		}
	}

	pub fn remove_self_role(&mut self, role_id: i64) -> Result<&mut Self, &str> {
		match self.self_roles.binary_search(&role_id) {
			Ok(i) => {
				self.self_roles.remove(i);
				Ok(self)
			}
			Err(_) => Err("Self role doesn't exist"),
		}
	}

	pub fn add_hydrate(&mut self, user_id: i64) -> Result<&mut Self, &str> {
		match self.hydrate.binary_search(&user_id) {
			Ok(_) => Err("Member already exists"),
			Err(_) => {
				self.hydrate.push(user_id);
				Ok(self)
			}
		}
	}

	pub fn remove_hydrate(&mut self, user_id: i64) -> Result<&mut Self, &str> {
		match self.hydrate.binary_search(&user_id) {
			Ok(i) => {
				self.hydrate.remove(i);
				Ok(self)
			}
			Err(_) => Err("Member doesn't exist"),
		}
	}

	pub fn add_disabled_command(&mut self, command: String) -> Result<&mut Self, &str> {
		match self.disabled_commands.binary_search(&command) {
			Ok(_) => Err("Already disabled"),
			Err(_) => {
				self.disabled_commands.push(command.clone());
				Ok(self)
			}
		}
	}

	pub fn remove_disabled_command(&mut self, command: String) -> Result<&mut Self, &str> {
		match self.disabled_commands.binary_search(&command) {
			Ok(i) => {
				self.disabled_commands.remove(i);
				Ok(self)
			}
			Err(_) => Err("Isn't disabled"),
		}
	}

	pub fn add_custom_command(&mut self, name: String, reply: String) -> Result<&mut Self, &str> {
		if self.custom_commands.iter().any(|y| y.name == name) {
			return Err("Custom command already exists");
		}
		let cmd = CustomCommand::new(name, reply);
		self.custom_commands.push(cmd.clone());
		Ok(self)
	}

	pub fn remove_custom_command(&mut self, name: String) -> Result<&mut Self, &str> {
		for (i, y) in self.custom_commands.iter().enumerate() {
			if y.name == name {
				self.custom_commands.remove(i);
				return Ok(self);
			}
		}
		Err("Custom command doesn't exist")
	}

	pub fn add_trigger_phrase(
		&mut self,
		phrase: String,
		reply: String,
		emote: char,
	) -> Result<&mut Self, &str> {
		if self.trigger_phrases.iter().any(|y| y.phrase == phrase) {
			return Err("Custom command already exists");
		}
		let wp = TriggerPhrase::new(phrase, reply, emote);
		self.trigger_phrases.push(wp.clone());
		Ok(self)
	}

	pub fn remove_trigger_phrase(&mut self, phrase: String) -> Result<&mut Self, &str> {
		for (i, y) in self.trigger_phrases.iter().enumerate() {
			if y.phrase == phrase {
				self.trigger_phrases.remove(i);
				return Ok(self);
			}
		}
		Err("Custom command doesn't exist")
	}

	pub fn change_prefix(&mut self, prefix: String) -> Result<&mut Self, &str> {
		self.prefix = prefix.clone();
		Ok(self)
	}

	pub fn change_default_role(&mut self, role_id: i64) -> Result<&mut Self, &str> {
		self.default_role = role_id;
		Ok(self)
	}

	pub fn get_member(&self, member_id: i64) -> Result<Member, &str> {
		for m in self.members.clone() {
			if m.id == member_id {
				return Ok(m);
			}
		}
		Err("Member doesn't exist")
	}

	pub fn update_member(&mut self, member: Member) -> Result<&mut Self, &str> {
		for m in self.members.iter_mut() {
			if m.id == member.id {
				*m = member;
				return Ok(self);
			}
		}
		Err("Member doesn't exist")
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Member {
	pub id: i64,
	pub coins: i64,
	pub last_daily: i64,
}

impl Member {
	pub fn new(member_id: i64) -> Self {
		Member {
			coins: 0,
			id: member_id,
			last_daily: 0,
		}
	}

	pub fn update_coins(&mut self, c: i64) -> &mut Self {
		self.coins += c;
		self
	}

	pub fn update_last_daily(&mut self, l: i64) -> &mut Self {
		self.last_daily += l;
		self
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomCommand {
	pub name: String,
	pub reply: String,
}

impl CustomCommand {
	pub fn new(name: String, reply: String) -> Self {
		CustomCommand {
			name: name,
			reply: reply,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerPhrase {
	pub phrase: String,
	pub reply: String,
	pub emote: char,
}

impl TriggerPhrase {
	pub fn new(phrase: String, reply: String, emote: char) -> Self {
		TriggerPhrase {
			phrase: phrase,
			reply: reply,
			emote: emote,
		}
	}
}
