//! The StompStation's signal chain on firmware 2.x: the `root\app\router`
//! node (docs/_reference/stompstation-router.md).
//!
//! The value is one row of strings. Each position holds a block's node path
//! or nothing, and between two neighbouring positions a connector says
//! whether they run in series (`"s"`) or in parallel (`"p"`). A run of
//! positions joined by `"p"` is one parallel bank: every position in it is a
//! branch that gets the bank's input, and the branches are summed into the
//! next position.
//!
//! The pedal normalises every write and answers it by echoing the request,
//! whatever it made of it. [`Router::apply`] predicts what it makes of one;
//! the value to show is still always the one read back afterwards. The edits
//! below compute a whole new row from the current one, which is what gets
//! written, once.
//!
//! Positions count from 0 here, as the row indexes them; the messages meant
//! for people count them from 1, as the pedal's screen does.

use std::ops::Range;

use serde_json::Value;

use crate::NodeDescription;

/// Where the router lives in the node tree.
pub const PATH: &str = "root\\app\\router";

/// How two neighbouring positions are joined.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Link {
    Series,
    Parallel,
}

impl Link {
    /// The connector as the row spells it.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Series => "s",
            Self::Parallel => "p",
        }
    }

    /// A connector as the pedal reads one it is sent: lower-cased, `"p"` is
    /// parallel and anything else is series.
    pub fn read(text: &str) -> Self {
        if text.eq_ignore_ascii_case("p") {
            Self::Parallel
        } else {
            Self::Series
        }
    }
}

/// Why a JSON value is not a router row.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RouterError {
    #[error("a router value is a list holding exactly one row")]
    NotOneRow,
    #[error("a router row needs at least one position")]
    Empty,
    #[error("a router row holds 2C - 1 cells for C positions, so it cannot have {0}")]
    EvenLength(usize),
    #[error("cell {0} of the router row is not a string")]
    NotAString(usize),
    #[error("cell {0} of the fixed row is not true or false")]
    NotABoolean(usize),
    #[error("the router row has {actual} cells where the pedal's has {expected}")]
    WrongLength { expected: usize, actual: usize },
}

/// Why an edit cannot be made.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum EditError {
    #[error("position {} is not in the chain", .0 + 1)]
    OutOfRange(usize),
    #[error("position {} is fixed", .0 + 1)]
    FixedPosition(usize),
    #[error("position {} already holds a block", .0 + 1)]
    Occupied(usize),
    #[error("position {} is empty", .0 + 1)]
    Empty(usize),
    #[error("{block} is already in position {}", .at + 1)]
    AlreadyPlaced { block: String, at: usize },
    #[error("there is no connector before position {}", .0 + 1)]
    NoConnector(usize),
    #[error("the connector before position {} is fixed", .0 + 1)]
    FixedConnector(usize),
    #[error("no block was named")]
    NoBlock,
    #[error("that changes nothing")]
    Unchanged,
}

/// One row of positions and the connectors between them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Router {
    /// What each position holds: a block's node path, `""` for nothing, or
    /// whatever else was written, which the pedal stores and treats as
    /// nothing.
    blocks: Vec<String>,
    /// `links[c - 1]` joins position `c - 1` and position `c`.
    links: Vec<Link>,
}

impl Router {
    /// A row from its positions and the connectors between them; `None`
    /// unless there is one connector fewer than positions, and at least one
    /// position.
    pub fn new(blocks: Vec<String>, links: Vec<Link>) -> Option<Self> {
        (!blocks.is_empty() && links.len() + 1 == blocks.len()).then_some(Self { blocks, links })
    }

    /// The row in a router value: a list holding one row of 2C - 1 strings.
    /// Connectors are read as the pedal reads them.
    pub fn from_value(value: &Value) -> Result<Self, RouterError> {
        let row = one_row(value)?;
        let mut cells = Vec::with_capacity(row.len());
        for (index, cell) in row.iter().enumerate() {
            cells.push(cell.as_str().ok_or(RouterError::NotAString(index))?);
        }
        Ok(Self {
            blocks: cells
                .iter()
                .step_by(2)
                .map(|cell| (*cell).to_owned())
                .collect(),
            links: cells
                .iter()
                .skip(1)
                .step_by(2)
                .map(|cell| Link::read(cell))
                .collect(),
        })
    }

    /// The value to write: a list holding the one row.
    pub fn to_value(&self) -> Value {
        let mut row = Vec::with_capacity(self.cells());
        for (position, block) in self.blocks.iter().enumerate() {
            if position > 0 {
                row.push(Value::from(self.links[position - 1].as_str()));
            }
            row.push(Value::from(block.as_str()));
        }
        Value::Array(vec![Value::Array(row)])
    }

    /// How many positions the row has: 14 on a PRO, 5 on a CORE.
    pub fn positions(&self) -> usize {
        self.blocks.len()
    }

    /// How many strings the row has.
    pub fn cells(&self) -> usize {
        2 * self.blocks.len() - 1
    }

    /// What a position holds, `None` when it holds nothing at all.
    pub fn block(&self, position: usize) -> Option<&str> {
        self.blocks
            .get(position)
            .map(String::as_str)
            .filter(|block| !block.is_empty())
    }

    /// The connector before a position, which joins it to the one before.
    pub fn link(&self, position: usize) -> Option<Link> {
        position
            .checked_sub(1)
            .and_then(|index| self.links.get(index))
            .copied()
    }

    /// Where a block is, when it is in the chain.
    pub fn position_of(&self, block: &str) -> Option<usize> {
        if block.is_empty() {
            return None;
        }
        self.blocks.iter().position(|held| held == block)
    }

    /// Whether a block is in the chain.
    pub fn contains(&self, block: &str) -> bool {
        self.position_of(block).is_some()
    }

    /// Every position that holds something, in order.
    pub fn occupied(&self) -> impl Iterator<Item = (usize, &str)> {
        self.blocks
            .iter()
            .enumerate()
            .filter(|(_, block)| !block.is_empty())
            .map(|(position, block)| (position, block.as_str()))
    }

    /// The chain in the order the signal meets it: each run of positions
    /// joined in parallel as one range, and each position between them as a
    /// range of its own. A range longer than one is a parallel bank.
    pub fn stages(&self) -> Vec<Range<usize>> {
        let mut stages = Vec::new();
        let mut start = 0;
        for position in 1..self.positions() {
            if self.links[position - 1] == Link::Series {
                stages.push(start..position);
                start = position;
            }
        }
        stages.push(start..self.positions());
        stages
    }

    /// The stage a position is in: its parallel bank, or itself alone.
    pub fn bank(&self, position: usize) -> Range<usize> {
        self.stages()
            .into_iter()
            .find(|stage| stage.contains(&position))
            .unwrap_or(position..position + 1)
    }

    /// What the pedal makes of `requested` written over this row, in the
    /// order its firmware applies its rules: a value that is not one row of
    /// as many strings changes nothing; connectors are lower-cased and
    /// anything but `"p"` becomes `"s"`; fixed cells keep what they hold;
    /// block strings are stored unchecked; and a block written twice keeps
    /// its later position, except that a copy of a fixed block is always the
    /// one dropped. A request that changes nothing gives this row back.
    pub fn apply(&self, fixed: &Fixed, requested: &Value) -> Router {
        let Ok(row) = one_row(requested) else {
            return self.clone();
        };
        if row.len() != self.cells() {
            return self.clone();
        }
        let Some(cells) = row.iter().map(Value::as_str).collect::<Option<Vec<_>>>() else {
            return self.clone();
        };
        let mut blocks: Vec<String> = cells
            .iter()
            .step_by(2)
            .map(|cell| (*cell).to_owned())
            .collect();
        let mut links: Vec<Link> = cells
            .iter()
            .skip(1)
            .step_by(2)
            .map(|cell| Link::read(cell))
            .collect();
        for (position, block) in blocks.iter_mut().enumerate() {
            if fixed.position(position) {
                block.clone_from(&self.blocks[position]);
            }
        }
        for (index, link) in links.iter_mut().enumerate() {
            if fixed.link(index + 1) {
                *link = self.links[index];
            }
        }
        for position in 0..blocks.len() {
            if blocks[position].is_empty() || fixed.position(position) {
                continue;
            }
            let later = blocks[position + 1..]
                .iter()
                .any(|other| *other == blocks[position]);
            let pinned = (0..blocks.len()).any(|other| {
                other != position && fixed.position(other) && blocks[other] == blocks[position]
            });
            if later || pinned {
                blocks[position].clear();
            }
        }
        Router { blocks, links }
    }

    /// Put a block that is not in the chain into an empty position.
    pub fn place(&self, fixed: &Fixed, block: &str, at: usize) -> Result<Router, EditError> {
        self.check_movable(fixed, at)?;
        if block.is_empty() {
            return Err(EditError::NoBlock);
        }
        if !self.blocks[at].is_empty() {
            return Err(EditError::Occupied(at));
        }
        if let Some(held) = self.position_of(block) {
            return Err(EditError::AlreadyPlaced {
                block: block.to_owned(),
                at: held,
            });
        }
        let mut next = self.clone();
        next.blocks[at] = block.to_owned();
        Ok(next)
    }

    /// Put a block where another is, which leaves the chain. A block already
    /// in the chain leaves its old position empty.
    pub fn replace(&self, fixed: &Fixed, at: usize, block: &str) -> Result<Router, EditError> {
        self.check_movable(fixed, at)?;
        if block.is_empty() {
            return Err(EditError::NoBlock);
        }
        if self.blocks[at].is_empty() {
            return Err(EditError::Empty(at));
        }
        if self.blocks[at] == block {
            return Err(EditError::Unchanged);
        }
        let mut next = self.clone();
        if let Some(held) = self.position_of(block) {
            if fixed.position(held) {
                return Err(EditError::FixedPosition(held));
            }
            next.blocks[held].clear();
        }
        next.blocks[at] = block.to_owned();
        Ok(next)
    }

    /// Empty a position. Its connectors stay as they are.
    pub fn remove(&self, fixed: &Fixed, at: usize) -> Result<Router, EditError> {
        self.check_movable(fixed, at)?;
        if self.blocks[at].is_empty() {
            return Err(EditError::Empty(at));
        }
        let mut next = self.clone();
        next.blocks[at].clear();
        Ok(next)
    }

    /// Empty a position and run what is left of its bank in series: where
    /// an empty branch passes the dry signal (2.2.6), a bank with a branch
    /// taken out would otherwise mix the dry signal in.
    pub fn remove_in_series(&self, fixed: &Fixed, at: usize) -> Result<Router, EditError> {
        let bank = self.bank(at);
        let mut next = self.remove(fixed, at)?;
        for position in bank.start + 1..bank.end {
            if !fixed.link(position) {
                next.links[position - 1] = Link::Series;
            }
        }
        Ok(next)
    }

    /// Exchange what two positions hold. The connectors stay with the
    /// positions.
    pub fn swap(&self, fixed: &Fixed, from: usize, to: usize) -> Result<Router, EditError> {
        self.check_movable(fixed, from)?;
        self.check_movable(fixed, to)?;
        if from == to || self.blocks[from] == self.blocks[to] {
            return Err(EditError::Unchanged);
        }
        let mut next = self.clone();
        next.blocks.swap(from, to);
        Ok(next)
    }

    /// Join a position to the one before it in series or in parallel.
    pub fn connect(&self, fixed: &Fixed, position: usize, link: Link) -> Result<Router, EditError> {
        if position == 0 || position >= self.positions() {
            return Err(EditError::NoConnector(position));
        }
        if fixed.link(position) {
            return Err(EditError::FixedConnector(position));
        }
        if self.links[position - 1] == link {
            return Err(EditError::Unchanged);
        }
        let mut next = self.clone();
        next.links[position - 1] = link;
        Ok(next)
    }

    /// The chain with every position that is not fixed emptied and every
    /// connector in series, as the pedal's Clear Chain leaves it.
    pub fn cleared(&self, fixed: &Fixed) -> Router {
        let mut next = self.clone();
        for (position, block) in next.blocks.iter_mut().enumerate() {
            if !fixed.position(position) {
                block.clear();
            }
        }
        for (index, link) in next.links.iter_mut().enumerate() {
            if !fixed.link(index + 1) {
                *link = Link::Series;
            }
        }
        next
    }

    fn check_movable(&self, fixed: &Fixed, position: usize) -> Result<(), EditError> {
        if position >= self.positions() {
            Err(EditError::OutOfRange(position))
        } else if fixed.position(position) {
            Err(EditError::FixedPosition(position))
        } else {
            Ok(())
        }
    }
}

/// Which cells of the row a write cannot change, interleaved like the row:
/// even cells are positions, odd ones connectors.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Fixed {
    cells: Vec<bool>,
}

impl Fixed {
    /// The `fixed` row a browse gives: a list holding one row of booleans.
    pub fn from_value(value: &Value) -> Result<Self, RouterError> {
        let row = one_row(value)?;
        let cells = row
            .iter()
            .enumerate()
            .map(|(index, cell)| cell.as_bool().ok_or(RouterError::NotABoolean(index)))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { cells })
    }

    /// Nothing fixed, for a row of `cells` strings.
    pub fn none(cells: usize) -> Self {
        Self {
            cells: vec![false; cells],
        }
    }

    /// Everything fixed, for a row of `cells` strings.
    pub fn all(cells: usize) -> Self {
        Self {
            cells: vec![true; cells],
        }
    }

    /// Whether a position keeps its block whatever a write says.
    pub fn position(&self, position: usize) -> bool {
        self.cells.get(2 * position).copied().unwrap_or(false)
    }

    /// Whether the connector before a position keeps its value.
    pub fn link(&self, position: usize) -> bool {
        position
            .checked_mul(2)
            .and_then(|cell| cell.checked_sub(1))
            .and_then(|cell| self.cells.get(cell))
            .copied()
            .unwrap_or(false)
    }

    /// How many cells it covers.
    pub fn cells(&self) -> usize {
        self.cells.len()
    }
}

/// What a browse says of the router: the chain now, the default chain, and
/// what a write cannot change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouterNode {
    pub value: Router,
    /// The chain `root\app\output\d_chain` restores, which every preset
    /// without a router record loads.
    pub default: Option<Router>,
    pub fixed: Fixed,
}

impl RouterNode {
    /// From the router's browsed description. A fixed row that does not
    /// match the value's length is taken as everything fixed, so nothing is
    /// offered that the pedal might not take; a missing one as nothing
    /// fixed.
    pub fn from_description(description: &NodeDescription) -> Result<Self, RouterError> {
        let value = Router::from_value(description.value.as_ref().ok_or(RouterError::NotOneRow)?)?;
        let default = description
            .default
            .as_ref()
            .and_then(|default| Router::from_value(default).ok())
            .filter(|default| default.cells() == value.cells());
        let fixed = match description.extra.get("fixed") {
            None => Fixed::none(value.cells()),
            Some(fixed) => Fixed::from_value(fixed)
                .ok()
                .filter(|fixed| fixed.cells() == value.cells())
                .unwrap_or_else(|| Fixed::all(value.cells())),
        };
        Ok(Self {
            value,
            default,
            fixed,
        })
    }

    /// The chain the pedal holds after the default is loaded over this one.
    pub fn defaulted(&self) -> Option<Router> {
        let default = self.default.as_ref()?;
        Some(self.value.apply(&self.fixed, &default.to_value()))
    }
}

/// The one row of a router or fixed value, with an odd number of cells.
fn one_row(value: &Value) -> Result<&Vec<Value>, RouterError> {
    let rows = value.as_array().ok_or(RouterError::NotOneRow)?;
    let [row] = rows.as_slice() else {
        return Err(RouterError::NotOneRow);
    };
    let row = row.as_array().ok_or(RouterError::NotOneRow)?;
    if row.is_empty() {
        return Err(RouterError::Empty);
    }
    if row.len() % 2 == 0 {
        return Err(RouterError::EvenLength(row.len()));
    }
    Ok(row)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const GATE: &str = "root\\app\\gate";
    const PITCH: &str = "root\\app\\pitch";
    const AMP: &str = "root\\app\\amp";
    const IR: &str = "root\\app\\ir";
    const MOD: &str = "root\\app\\mod";
    const DELAY: &str = "root\\app\\delay";
    const DELAY2: &str = "root\\app\\delay2";
    const REVERB: &str = "root\\app\\reverb";
    const CHORUS: &str = "root\\app\\chorus";

    /// The chain every preset without a router record loads, as a 2.0.10
    /// pedal's schema gives it.
    fn default_value() -> Value {
        json!([[
            "root\\app\\gate",
            "s",
            "root\\app\\pitch",
            "s",
            "root\\app\\exp",
            "s",
            "root\\app\\comp",
            "s",
            "root\\app\\mod_pre",
            "s",
            "root\\app\\drive",
            "s",
            "root\\app\\amp",
            "s",
            "root\\app\\ir",
            "s",
            "root\\app\\eq",
            "s",
            "",
            "s",
            "root\\app\\mod",
            "s",
            "root\\app\\delay",
            "s",
            "root\\app\\reverb",
            "s",
            ""
        ]])
    }

    fn default_chain() -> Router {
        Router::from_value(&default_value()).unwrap()
    }

    /// The PRO's fixed cells: the amp (6), the delay (11) and the reverb
    /// (12), interleaved as the pedal sends them.
    fn pro_fixed() -> Fixed {
        let mut cells = vec![false; 27];
        for position in [6, 11, 12] {
            cells[2 * position] = true;
        }
        Fixed::from_value(&json!([cells])).unwrap()
    }

    /// A row with one cell changed.
    fn with(router: &Router, cell: usize, text: &str) -> Value {
        let mut value = router.to_value();
        value[0][cell] = Value::from(text);
        value
    }

    #[test]
    fn the_default_chain_reads_and_writes_back_unchanged() {
        let chain = default_chain();
        assert_eq!(chain.positions(), 14);
        assert_eq!(chain.cells(), 27);
        assert_eq!(chain.to_value(), default_value());
        assert_eq!(chain.block(0), Some(GATE));
        assert_eq!(chain.block(9), None, "position 10 is empty");
        assert_eq!(chain.block(13), None);
        assert_eq!(chain.block(14), None, "past the end");
        assert_eq!(chain.position_of(AMP), Some(6));
        assert!(!chain.contains(CHORUS));
        assert!(!chain.contains(""), "nothing is never in the chain");
        assert_eq!(chain.link(0), None, "nothing comes before the first");
        assert_eq!(chain.link(1), Some(Link::Series));
        assert_eq!(chain.link(14), None);
        assert_eq!(chain.occupied().count(), 12);
    }

    #[test]
    fn a_row_takes_its_size_from_its_length() {
        // A CORE has five positions.
        let core =
            Router::from_value(&json!([[GATE, "s", "", "p", AMP, "s", "", "s", ""]])).unwrap();
        assert_eq!(core.positions(), 5);
        assert_eq!(core.link(2), Some(Link::Parallel));
        let single = Router::from_value(&json!([[AMP]])).unwrap();
        assert_eq!(single.positions(), 1);
        assert_eq!(single.stages(), vec![0..1]);
        assert_eq!(single.to_value(), json!([[AMP]]));
    }

    #[test]
    fn a_value_that_is_not_one_row_of_strings_is_refused() {
        assert_eq!(
            Router::from_value(&json!("gate")),
            Err(RouterError::NotOneRow)
        );
        assert_eq!(Router::from_value(&json!([])), Err(RouterError::NotOneRow));
        assert_eq!(
            Router::from_value(&json!([[GATE], [AMP]])),
            Err(RouterError::NotOneRow)
        );
        assert_eq!(
            Router::from_value(&json!([GATE])),
            Err(RouterError::NotOneRow)
        );
        assert_eq!(Router::from_value(&json!([[]])), Err(RouterError::Empty));
        assert_eq!(
            Router::from_value(&json!([[GATE, "s"]])),
            Err(RouterError::EvenLength(2))
        );
        assert_eq!(
            Router::from_value(&json!([[GATE, 1, AMP]])),
            Err(RouterError::NotAString(1))
        );
        assert_eq!(
            Router::from_value(&json!({"value": [[GATE]]})),
            Err(RouterError::NotOneRow)
        );
    }

    #[test]
    fn connectors_are_read_as_the_pedal_reads_them() {
        assert_eq!(Link::read("p"), Link::Parallel);
        assert_eq!(Link::read("P"), Link::Parallel);
        for series in ["s", "S", "", "x", "pp", "parallel", " p"] {
            assert_eq!(Link::read(series), Link::Series, "{series:?}");
        }
        let read = Router::from_value(&json!([[GATE, "P", AMP, "q", IR]])).unwrap();
        assert_eq!(read.to_value(), json!([[GATE, "p", AMP, "s", IR]]));
    }

    #[test]
    fn a_router_needs_one_connector_fewer_than_positions() {
        assert!(Router::new(vec![], vec![]).is_none());
        assert!(Router::new(vec![GATE.into()], vec![Link::Series]).is_none());
        let pair = Router::new(vec![GATE.into(), AMP.into()], vec![Link::Parallel]).unwrap();
        assert_eq!(pair.to_value(), json!([[GATE, "p", AMP]]));
    }

    #[test]
    fn positions_joined_in_parallel_are_one_bank() {
        let chain = default_chain();
        assert_eq!(chain.stages().len(), 14, "all in series");
        assert!(chain.stages().iter().all(|stage| stage.len() == 1));

        // Delay and reverb in parallel after the modulation.
        let paired = Router::from_value(&with(&chain, 23, "p")).unwrap();
        let stages = paired.stages();
        assert_eq!(stages.len(), 13);
        assert!(stages.contains(&(11..13)));
        assert_eq!(paired.bank(12), 11..13);
        assert_eq!(paired.bank(13), 13..14);

        // Three in parallel, and a second bank.
        let mut value = with(&chain, 21, "p");
        value[0][23] = json!("p");
        value[0][1] = json!("p");
        let banks = Router::from_value(&value).unwrap();
        assert_eq!(banks.stages()[0], 0..2);
        assert!(banks.stages().contains(&(10..13)));
        assert_eq!(banks.bank(11), 10..13);

        // Everything in parallel is one bank.
        let mut all = chain.to_value();
        for cell in (1..27).step_by(2) {
            all[0][cell] = json!("p");
        }
        assert_eq!(Router::from_value(&all).unwrap().stages(), vec![0..14]);
        assert_eq!(chain.bank(20), 20..21, "past the end, alone");
    }

    #[test]
    fn a_value_of_the_wrong_shape_changes_nothing() {
        let chain = default_chain();
        let fixed = pro_fixed();
        for wrong in [
            json!(null),
            json!("root\\app\\gate"),
            json!([]),
            json!([[]]),
            json!([[GATE, "s", AMP]]),
            json!([[GATE], [AMP]]),
            Value::Array(vec![Value::Array(vec![json!(""); 25])]),
            Value::Array(vec![Value::Array(vec![json!(""); 29])]),
            Value::Array(vec![Value::Array(vec![json!(""); 28])]),
        ] {
            assert_eq!(chain.apply(&fixed, &wrong), chain, "{wrong}");
        }
        let mut number = chain.to_value();
        number[0][4] = json!(4);
        assert_eq!(chain.apply(&fixed, &number), chain, "a number in the row");
        let mut flat = chain.to_value()[0].clone();
        flat[0] = json!("");
        assert_eq!(chain.apply(&fixed, &flat), chain, "a row without its list");
    }

    #[test]
    fn written_connectors_are_lower_cased_and_anything_but_p_is_series() {
        let chain = default_chain();
        let fixed = pro_fixed();
        let mut value = chain.to_value();
        value[0][1] = json!("P");
        value[0][3] = json!("parallel");
        value[0][5] = json!("");
        value[0][7] = json!("S");
        let applied = chain.apply(&fixed, &value);
        assert_eq!(applied.link(1), Some(Link::Parallel));
        assert_eq!(applied.link(2), Some(Link::Series));
        assert_eq!(applied.link(3), Some(Link::Series));
        assert_eq!(applied.link(4), Some(Link::Series));
        assert_eq!(applied.to_value()[0][1], json!("p"));
        assert_eq!(applied.to_value()[0][7], json!("s"));
    }

    #[test]
    fn fixed_positions_keep_their_block_whatever_a_write_says() {
        let chain = default_chain();
        let fixed = pro_fixed();
        // Emptied: the amp stays.
        assert_eq!(chain.apply(&fixed, &with(&chain, 12, "")), chain);
        // Something else written there: the amp stays, and so does the delay
        // in its own fixed position.
        assert_eq!(chain.apply(&fixed, &with(&chain, 12, CHORUS)), chain);
        // A fixed block moved away: it stays where it is fixed, and the copy
        // written elsewhere is dropped.
        let mut moved = chain.to_value();
        moved[0][12] = json!("");
        moved[0][18] = json!(AMP);
        assert_eq!(chain.apply(&fixed, &moved), chain);
        // Other cells still change around them.
        let applied = chain.apply(&fixed, &with(&chain, 0, CHORUS));
        assert_eq!(applied.block(0), Some(CHORUS));
        assert_eq!(applied.block(6), Some(AMP));
    }

    #[test]
    fn a_fixed_connector_keeps_its_value() {
        let chain = Router::from_value(&json!([[GATE, "s", AMP, "p", IR]])).unwrap();
        let fixed = Fixed::from_value(&json!([[false, true, false, false, false]])).unwrap();
        let applied = chain.apply(&fixed, &json!([[GATE, "p", AMP, "s", IR]]));
        assert_eq!(applied.link(1), Some(Link::Series), "fixed");
        assert_eq!(applied.link(2), Some(Link::Series), "not fixed");
        assert!(fixed.link(1));
        assert!(!fixed.link(0));
        assert!(!fixed.link(2));
        assert!(!fixed.link(9), "past the end");
    }

    #[test]
    fn a_block_written_twice_keeps_its_later_position() {
        let chain = default_chain();
        let fixed = pro_fixed();
        // Chorus in positions 1 and 10: it ends up in 10.
        let mut twice = with(&chain, 2, CHORUS);
        twice[0][18] = json!(CHORUS);
        let applied = chain.apply(&fixed, &twice);
        assert_eq!(applied.block(1), None);
        assert_eq!(applied.block(9), Some(CHORUS));
        // Three times: the last.
        twice[0][26] = json!(CHORUS);
        let applied = chain.apply(&fixed, &twice);
        assert_eq!(applied.position_of(CHORUS), Some(13));
        assert_eq!(applied.occupied().filter(|(_, b)| *b == CHORUS).count(), 1);
        // The gate written after itself moves to the later position.
        let applied = chain.apply(&fixed, &with(&chain, 18, GATE));
        assert_eq!(applied.block(0), None);
        assert_eq!(applied.block(9), Some(GATE));
        // Empty positions are never duplicates of each other.
        assert_eq!(chain.apply(&fixed, &chain.to_value()), chain);
    }

    #[test]
    fn a_copy_of_a_fixed_block_is_always_the_one_dropped() {
        let chain = default_chain();
        let fixed = pro_fixed();
        // Later than the fixed reverb: dropped although it is later.
        let applied = chain.apply(&fixed, &with(&chain, 26, REVERB));
        assert_eq!(applied.block(13), None);
        assert_eq!(applied.block(12), Some(REVERB));
        // Earlier: dropped too.
        let applied = chain.apply(&fixed, &with(&chain, 0, DELAY));
        assert_eq!(applied.block(0), None);
        assert_eq!(applied.block(11), Some(DELAY));
    }

    #[test]
    fn unknown_block_strings_are_stored_as_written() {
        let chain = default_chain();
        let fixed = pro_fixed();
        let applied = chain.apply(&fixed, &with(&chain, 18, "root\\app\\pcm42"));
        assert_eq!(applied.block(9), Some("root\\app\\pcm42"));
        let applied = chain.apply(&fixed, &with(&chain, 18, "anything at all"));
        assert_eq!(applied.block(9), Some("anything at all"));
        // Written twice, unknown strings are duplicates like any other.
        let mut twice = with(&chain, 18, "x");
        twice[0][26] = json!("x");
        let applied = chain.apply(&fixed, &twice);
        assert_eq!(applied.block(9), None);
        assert_eq!(applied.block(13), Some("x"));
    }

    #[test]
    fn a_write_that_changes_nothing_gives_the_same_chain() {
        let chain = default_chain();
        assert_eq!(chain.apply(&pro_fixed(), &chain.to_value()), chain);
        assert_eq!(chain.apply(&Fixed::none(27), &chain.to_value()), chain);
    }

    #[test]
    fn a_block_goes_into_an_empty_position_once() {
        let chain = default_chain();
        let fixed = pro_fixed();
        let placed = chain.place(&fixed, CHORUS, 9).unwrap();
        assert_eq!(placed.block(9), Some(CHORUS));
        assert_eq!(chain.place(&fixed, CHORUS, 0), Err(EditError::Occupied(0)));
        assert_eq!(
            placed.place(&fixed, CHORUS, 13),
            Err(EditError::AlreadyPlaced {
                block: CHORUS.into(),
                at: 9
            })
        );
        assert_eq!(chain.place(&fixed, "", 9), Err(EditError::NoBlock));
        assert_eq!(
            chain.place(&fixed, CHORUS, 14),
            Err(EditError::OutOfRange(14))
        );
        // A fixed position is never offered, empty or not.
        let empty_fixed = Fixed::from_value(&json!([[false, false, true]])).unwrap();
        let pair = Router::from_value(&json!([[GATE, "s", ""]])).unwrap();
        assert_eq!(
            pair.place(&empty_fixed, AMP, 1),
            Err(EditError::FixedPosition(1))
        );
    }

    #[test]
    fn replacing_takes_the_old_block_out_of_the_chain() {
        let chain = default_chain();
        let fixed = pro_fixed();
        let replaced = chain.replace(&fixed, 1, CHORUS).unwrap();
        assert_eq!(replaced.block(1), Some(CHORUS));
        assert!(!replaced.contains(PITCH));
        // A block already in the chain leaves its old position empty.
        let moved = chain.replace(&fixed, 1, MOD).unwrap();
        assert_eq!(moved.block(1), Some(MOD));
        assert_eq!(moved.block(10), None);
        assert!(!moved.contains(PITCH));
        // Not a fixed block, not into a fixed position, not into nothing.
        assert_eq!(
            chain.replace(&fixed, 1, AMP),
            Err(EditError::FixedPosition(6))
        );
        assert_eq!(
            chain.replace(&fixed, 6, CHORUS),
            Err(EditError::FixedPosition(6))
        );
        assert_eq!(chain.replace(&fixed, 9, CHORUS), Err(EditError::Empty(9)));
        assert_eq!(chain.replace(&fixed, 1, PITCH), Err(EditError::Unchanged));
        assert_eq!(chain.replace(&fixed, 1, ""), Err(EditError::NoBlock));
        // An unknown string acts empty, but it is replaced, not placed over.
        let unknown = chain.apply(&fixed, &with(&chain, 18, "root\\app\\pcm42"));
        assert_eq!(
            unknown.place(&fixed, CHORUS, 9),
            Err(EditError::Occupied(9))
        );
        assert_eq!(
            unknown.replace(&fixed, 9, CHORUS).unwrap().block(9),
            Some(CHORUS)
        );
    }

    #[test]
    fn removing_empties_the_position_and_keeps_its_connectors() {
        let chain = default_chain();
        let fixed = pro_fixed();
        let paired = chain.apply(&fixed, &with(&chain, 21, "p"));
        let removed = paired.remove(&fixed, 10).unwrap();
        assert_eq!(removed.block(10), None);
        assert_eq!(removed.link(11), Some(Link::Parallel), "the bank stays");
        assert_eq!(chain.remove(&fixed, 6), Err(EditError::FixedPosition(6)));
        assert_eq!(chain.remove(&fixed, 9), Err(EditError::Empty(9)));
        assert_eq!(chain.remove(&fixed, 20), Err(EditError::OutOfRange(20)));
    }

    #[test]
    fn removing_a_branch_can_run_the_rest_of_its_bank_in_series() {
        let chain = default_chain();
        let fixed = pro_fixed();
        // Modulation, delay and reverb in parallel.
        let mut value = with(&chain, 21, "p");
        value[0][23] = json!("p");
        let bank = chain.apply(&fixed, &value);
        assert_eq!(bank.bank(11), 10..13);
        let removed = bank.remove_in_series(&fixed, 10).unwrap();
        assert_eq!(removed.block(10), None);
        assert_eq!(removed.link(11), Some(Link::Series));
        assert_eq!(removed.link(12), Some(Link::Series));
        assert!(removed.stages().iter().all(|stage| stage.len() == 1));
        // Outside a bank it is only a removal.
        assert_eq!(chain.remove_in_series(&fixed, 1), chain.remove(&fixed, 1));
        assert_eq!(
            bank.remove_in_series(&fixed, 11),
            Err(EditError::FixedPosition(11))
        );
    }

    #[test]
    fn swapping_moves_blocks_and_leaves_connectors_with_their_positions() {
        let chain = default_chain();
        let fixed = pro_fixed();
        let paired = chain.apply(&fixed, &with(&chain, 19, "p"));
        // The modulation into the empty position before it, which is in a
        // bank with it.
        let swapped = paired.swap(&fixed, 10, 9).unwrap();
        assert_eq!(swapped.block(9), Some(MOD));
        assert_eq!(swapped.block(10), None);
        assert_eq!(swapped.link(10), Some(Link::Parallel));
        // Two blocks trade places.
        let traded = chain.swap(&fixed, 0, 1).unwrap();
        assert_eq!(traded.block(0), Some(PITCH));
        assert_eq!(traded.block(1), Some(GATE));
        assert_eq!(chain.swap(&fixed, 0, 6), Err(EditError::FixedPosition(6)));
        assert_eq!(chain.swap(&fixed, 11, 0), Err(EditError::FixedPosition(11)));
        assert_eq!(chain.swap(&fixed, 0, 0), Err(EditError::Unchanged));
        assert_eq!(
            chain.swap(&fixed, 9, 13),
            Err(EditError::Unchanged),
            "both empty"
        );
        assert_eq!(chain.swap(&fixed, 0, 14), Err(EditError::OutOfRange(14)));
    }

    #[test]
    fn connecting_joins_a_position_to_the_one_before() {
        let chain = default_chain();
        let fixed = pro_fixed();
        let paired = chain.connect(&fixed, 12, Link::Parallel).unwrap();
        assert_eq!(paired.link(12), Some(Link::Parallel));
        assert_eq!(paired.bank(11), 11..13);
        assert_eq!(paired.connect(&fixed, 12, Link::Series).unwrap(), chain);
        assert_eq!(
            chain.connect(&fixed, 12, Link::Series),
            Err(EditError::Unchanged)
        );
        assert_eq!(
            chain.connect(&fixed, 0, Link::Parallel),
            Err(EditError::NoConnector(0))
        );
        assert_eq!(
            chain.connect(&fixed, 14, Link::Parallel),
            Err(EditError::NoConnector(14))
        );
        let pinned = Fixed::from_value(&json!([[false, true, false]])).unwrap();
        let pair = Router::from_value(&json!([[GATE, "s", AMP]])).unwrap();
        assert_eq!(
            pair.connect(&pinned, 1, Link::Parallel),
            Err(EditError::FixedConnector(1))
        );
    }

    #[test]
    fn clearing_keeps_only_what_is_fixed() {
        let chain = default_chain();
        let fixed = pro_fixed();
        let banked = chain.connect(&fixed, 12, Link::Parallel).unwrap();
        let cleared = banked.cleared(&fixed);
        assert_eq!(
            cleared.occupied().collect::<Vec<_>>(),
            vec![(6, AMP), (11, DELAY), (12, REVERB)]
        );
        assert!(cleared.stages().iter().all(|stage| stage.len() == 1));
    }

    /// Every edit computes what the pedal itself would make of it, so the
    /// row written is the row read back.
    #[test]
    fn every_edit_is_a_row_the_pedal_keeps_as_written() {
        let chain = default_chain();
        let fixed = pro_fixed();
        let banked = chain
            .connect(&fixed, 12, Link::Parallel)
            .and_then(|r| r.connect(&fixed, 11, Link::Parallel))
            .unwrap();
        let edits = [
            chain.place(&fixed, CHORUS, 9),
            chain.place(&fixed, DELAY2, 13),
            chain.replace(&fixed, 1, CHORUS),
            chain.replace(&fixed, 1, MOD),
            chain.remove(&fixed, 0),
            banked.remove(&fixed, 10),
            banked.remove_in_series(&fixed, 10),
            chain.swap(&fixed, 0, 9),
            chain.swap(&fixed, 2, 3),
            chain.connect(&fixed, 1, Link::Parallel),
            banked.connect(&fixed, 12, Link::Series),
            Ok(banked.cleared(&fixed)),
        ];
        for edit in edits {
            let edit = edit.unwrap();
            for start in [&chain, &banked] {
                assert_eq!(start.apply(&fixed, &edit.to_value()), edit);
            }
        }
    }

    #[test]
    fn a_fixed_row_reads_like_the_value() {
        let fixed = pro_fixed();
        assert_eq!(fixed.cells(), 27);
        assert!(fixed.position(6) && fixed.position(11) && fixed.position(12));
        assert!(!fixed.position(0) && !fixed.position(13) && !fixed.position(14));
        assert!((1..14).all(|position| !fixed.link(position)));
        assert_eq!(
            Fixed::from_value(&json!([[true, "no", false]])),
            Err(RouterError::NotABoolean(1))
        );
        assert_eq!(
            Fixed::from_value(&json!([[true, false]])),
            Err(RouterError::EvenLength(2))
        );
        assert!(Fixed::all(3).position(1) && Fixed::all(3).link(1));
        assert!(!Fixed::none(3).position(1));
    }

    #[test]
    fn a_browse_describes_the_chain_its_default_and_what_is_fixed() {
        let mut fixed_cells = vec![false; 27];
        for position in [6, 11, 12] {
            fixed_cells[2 * position] = true;
        }
        let description: NodeDescription = serde_json::from_value(json!({
            "type": "router", "desc": "router",
            "value": default_value(), "def": default_value(), "fixed": [fixed_cells],
        }))
        .unwrap();
        let node = RouterNode::from_description(&description).unwrap();
        assert_eq!(node.value, default_chain());
        assert_eq!(node.default.as_ref(), Some(&default_chain()));
        assert_eq!(node.fixed, pro_fixed());
        assert_eq!(node.defaulted(), Some(default_chain()));

        // A fixed row that does not fit leaves nothing editable; none at all
        // leaves everything.
        let mut odd = description.clone();
        odd.extra
            .insert("fixed".into(), json!([[true, false, true]]));
        let node = RouterNode::from_description(&odd).unwrap();
        assert!((0..14).all(|position| node.fixed.position(position)));
        let mut bare = description.clone();
        bare.extra.remove("fixed");
        bare.default = None;
        let node = RouterNode::from_description(&bare).unwrap();
        assert!((0..14).all(|position| !node.fixed.position(position)));
        assert_eq!(node.defaulted(), None);
        let mut missing = description;
        missing.value = None;
        assert_eq!(
            RouterNode::from_description(&missing),
            Err(RouterError::NotOneRow)
        );
    }

    #[test]
    fn loading_the_default_keeps_fixed_blocks_where_they_are() {
        let chain = default_chain();
        let fixed = pro_fixed();
        let edited = chain
            .replace(&fixed, 1, CHORUS)
            .and_then(|r| r.connect(&fixed, 12, Link::Parallel))
            .unwrap();
        let node = RouterNode {
            value: edited,
            default: Some(chain.clone()),
            fixed,
        };
        assert_eq!(node.defaulted(), Some(chain));
    }

    #[test]
    fn edit_messages_count_positions_from_one() {
        assert_eq!(
            EditError::FixedPosition(6).to_string(),
            "position 7 is fixed"
        );
        assert_eq!(
            EditError::AlreadyPlaced {
                block: CHORUS.into(),
                at: 9
            }
            .to_string(),
            "root\\app\\chorus is already in position 10"
        );
    }
}
