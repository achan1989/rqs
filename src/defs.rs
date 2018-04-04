// Copyright (C) 1996-1997 Id Software, Inc.
//
// This program is free software; you can redistribute it and/or
// modify it under the terms of the GNU General Public License
// as published by the Free Software Foundation; either version 2
// of the License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
//
// See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program; if not, write to the Free Software
// Foundation, Inc., 59 Temple Place - Suite 330, Boston, MA  02111-1307, USA.

// Modified by Adrian Chan, March 2018
// Small parts of quakedef.h

//! Some useful definitions.

/// Defines the base directory for the game.
pub const GAMENAME: &str = "id1";

/// The maximum number of command line arguments that will be accepted.
pub const MAX_NUM_ARGVS: usize = 50;
/// The `cmdline` cvar will report the command line arguments used, up to this
/// length.
pub const CMDLINE_LENGTH: usize = 256;
