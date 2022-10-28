/*
 * Rust Query API - A versatile API facade for the Hypixel Auction API
 * Copyright (c) 2022 kr45732
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */
use crate::statics::HTTP_CLIENT;
use serde::Serialize;
use std::error::Error;

#[derive(Debug, Serialize)]
pub struct EmbedBuilder {
    title: Option<String>,
    description: Option<String>,
    color: Option<i32>,
}

impl EmbedBuilder {
    pub fn new() -> Self {
        Self {
            title: None,
            description: None,
            color: None,
        }
    }

    pub fn title(&mut self, title: &str) -> &mut EmbedBuilder {
        self.title = Some(title.to_owned());
        self
    }

    pub fn description(&mut self, description: &str) -> &mut EmbedBuilder {
        self.description = Some(description.to_owned());
        self
    }

    pub fn color(&mut self, color: i32) -> &mut EmbedBuilder {
        self.color = Some(color);
        self
    }

    pub fn build(&mut self) -> Embed {
        Embed {
            title: self.title.clone(),
            description: self.description.clone(),
            color: self.color,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Embed {
    title: Option<String>,
    description: Option<String>,
    color: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct Message {
    content: Option<String>,
    embeds: Vec<Embed>,
}

impl Message {
    pub fn new() -> Self {
        Self {
            content: None,
            embeds: vec![],
        }
    }

    pub fn content(&mut self, content: &str) -> &mut Message {
        self.content = Some(content.to_owned());
        self
    }

    pub fn mention(&mut self, mention: bool) -> &mut Message {
        if mention {
            self.content("<@796791167366594592>");
        }
        self
    }

    pub fn embed<F>(&mut self, embed: F) -> &mut Message
    where
        F: Fn(&mut EmbedBuilder) -> &mut EmbedBuilder,
    {
        self.embeds.push((embed(&mut EmbedBuilder::new())).build());
        self
    }
}

pub struct Webhook {
    url: String,
}

impl Webhook {
    pub fn from_url(url: &str) -> Self {
        Self {
            url: url.to_owned(),
        }
    }

    pub async fn send<F>(&self, t: F) -> Result<(), Box<dyn Error>>
    where
        F: Fn(&mut Message) -> &mut Message,
    {
        let mut msg = Message::new();
        let message = t(&mut msg);
        HTTP_CLIENT.post(&self.url).body_json(&message)?.await?;
        Ok(())
    }
}
