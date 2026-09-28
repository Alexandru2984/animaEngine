//! Screen readers, over AT-SPI.
//!
//! egui describes each frame's widgets as an AccessKit tree. Something has
//! to publish that tree where screen readers look — AT-SPI, on the
//! accessibility bus — and carry their requests (focus this, press that)
//! back into egui. egui-winit can do it, but only once `init_accesskit` is
//! called, which this program never did, and the native Wayland path has
//! no winit at all: until 1.4 no screen reader could see anything on
//! either backend, whatever the docs and the Appearance toggle said (R53).
//!
//! One bridge serves both backends, straight on `accesskit_unix`, the crate
//! egui-winit would have used. Each egui renderer owns one. It costs a
//! thread and a session-bus connection watching `org.a11y.Status`; egui
//! builds no tree until an assistive technology asks for one.

use egui::accesskit::{
    ActionHandler, ActionRequest, ActivationHandler, DeactivationHandler, Node, NodeId, Rect, Role,
    Tree, TreeUpdate,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

/// Name the tree is published under — what a screen reader announces.
const APP_NAME: &str = "animaEngine";

/// Requests held for the next frame at most. A screen reader sends a few
/// per keystroke; anything past this is a misbehaving client on the bus,
/// and its excess is dropped rather than buffered without bound.
const MAX_PENDING_REQUESTS: usize = 256;

/// Root of the empty tree published while the setting is off. egui's node
/// ids are never zero, so it cannot collide with one of theirs.
const WITHHELD_ROOT: NodeId = NodeId(0);

/// Asks the frame loop for a frame, so the next one answers the reader.
type Wake = Arc<dyn Fn() + Send + Sync>;

/// State the adapter's thread writes and the frame loop reads.
#[derive(Default)]
struct Shared {
    /// A screen reader asked for the tree and has not gone away.
    listening: AtomicBool,
    /// Its requests since the last frame.
    requests: Mutex<Vec<ActionRequest>>,
}

/// The three AccessKit callbacks. `accesskit_unix` calls them on its own
/// thread; each only flips a flag or queues a request, then wakes the loop.
#[derive(Clone)]
struct Handler {
    shared: Arc<Shared>,
    wake: Wake,
}

impl ActivationHandler for Handler {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        // No tree yet: egui builds one from the next frame on, and the
        // adapter waits for that first update.
        self.shared.listening.store(true, Ordering::Release);
        (self.wake)();
        None
    }
}

impl ActionHandler for Handler {
    fn do_action(&mut self, request: ActionRequest) {
        let mut requests = self
            .shared
            .requests
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if requests.len() < MAX_PENDING_REQUESTS {
            requests.push(request);
        }
        drop(requests);
        (self.wake)();
    }
}

impl DeactivationHandler for Handler {
    fn deactivate_accessibility(&mut self) {
        self.shared.listening.store(false, Ordering::Release);
    }
}

/// One window's tree, published on AT-SPI.
pub struct ScreenReaderBridge {
    adapter: accesskit_unix::Adapter,
    shared: Arc<Shared>,
    /// Last values handed to the adapter, so an unchanged frame sends
    /// nothing.
    focused: Option<bool>,
    bounds: Option<[f64; 4]>,
    /// The empty tree went out since the setting was turned off.
    withheld: bool,
}

impl ScreenReaderBridge {
    /// Register with AT-SPI. `wake` must make the frame loop draw a frame
    /// soon; it is called from another thread.
    pub fn new(wake: impl Fn() + Send + Sync + 'static) -> Self {
        let shared = Arc::new(Shared::default());
        let handler = Handler {
            shared: Arc::clone(&shared),
            wake: Arc::new(wake),
        };
        let adapter = accesskit_unix::Adapter::new(handler.clone(), handler.clone(), handler);
        Self {
            adapter,
            shared,
            focused: None,
            bounds: None,
            withheld: false,
        }
    }

    /// Whether a screen reader is listening. egui should build its tree
    /// only then.
    pub fn is_listening(&self) -> bool {
        self.shared.listening.load(Ordering::Acquire)
    }

    /// The reader's pending requests, as egui input for this frame.
    pub fn drain_requests(&self) -> impl Iterator<Item = egui::Event> {
        std::mem::take(
            &mut *self
                .shared
                .requests
                .lock()
                .unwrap_or_else(PoisonError::into_inner),
        )
        .into_iter()
        .map(egui::Event::AccessKitActionRequest)
    }

    /// Publish a frame's tree. egui sends the whole tree every frame,
    /// which is what the adapter needs for its first one.
    ///
    /// With the setting off (`allowed` false) the reader gets the window
    /// and nothing in it, once: the last tree must not stay readable —
    /// the setting's hint promises that what is typed in the panels stops
    /// reaching the bus.
    pub fn publish(&mut self, allowed: bool, update: Option<TreeUpdate>) {
        if !self.is_listening() {
            // A reader that comes back gets the empty tree again.
            self.withheld = false;
            return;
        }
        if allowed {
            if let Some(update) = update {
                self.withheld = false;
                self.adapter.update_if_active(|| prepared(update));
            }
        } else if !self.withheld {
            self.withheld = true;
            self.adapter.update_if_active(withheld_tree);
        }
    }

    /// Whether the window has the keyboard. A screen reader follows focus
    /// only inside the active window.
    pub fn set_focused(&mut self, focused: bool) {
        if self.focused != Some(focused) {
            self.focused = Some(focused);
            self.adapter.update_window_focus_state(focused);
        }
    }

    /// Where the window's content sits on the screen, in physical pixels.
    /// A screen reader uses it to place what it reports; reading and
    /// navigating work without it.
    pub fn set_bounds(&mut self, x: f64, y: f64, width: f64, height: f64) {
        let bounds = [x, y, width, height];
        if self.bounds != Some(bounds) {
            self.bounds = Some(bounds);
            let rect = Rect::new(x, y, x + width, y + height);
            self.adapter.set_root_window_bounds(rect, rect);
        }
    }
}

/// Ready a frame's tree for the reader: named for the application and its
/// window, both of which egui leaves unnamed, with icon-font glyphs
/// dropped from what is read out, and without the nodes that then have
/// nothing to say.
fn prepared(mut update: TreeUpdate) -> TreeUpdate {
    let root = update.tree.as_mut().map(|tree| {
        tree.app_name = Some(APP_NAME.to_string());
        tree.toolkit_name = Some("egui".to_string());
        tree.root
    });
    for (id, node) in &mut update.nodes {
        if Some(*id) == root && node.label().is_none() {
            node.set_label(APP_NAME);
        }
        let emptied = without_icons(node);
        if is_decorative(node, emptied) {
            node.set_hidden();
            // A reader announces the focused node even when hidden. egui
            // focuses the ⚙ corner's backdrop once the button is clicked,
            // and "unknown" was all there was to say; the window says more.
            if update.focus == *id {
                if let Some(root) = root {
                    update.focus = root;
                }
            }
        }
    }
    update
}

/// What a reader gets while the setting is off: the window, named, and
/// nothing in it.
fn withheld_tree() -> TreeUpdate {
    prepared(TreeUpdate {
        nodes: vec![(WITHHELD_ROOT, Node::new(Role::Window))],
        tree: Some(Tree::new(WITHHELD_ROOT)),
        focus: WITHHELD_ROOT,
    })
}

/// Nodes a reader would stop on and have nothing to read: a label that was
/// only an icon, and the empty backdrops egui gives its floating areas (the
/// ⚙ corner, the tour card), which have no role, name or content.
fn is_decorative(node: &Node, emptied: bool) -> bool {
    let unnamed = node.label().is_none() && node.value().is_none();
    match node.role() {
        Role::Label => emptied && unnamed,
        Role::Unknown => unnamed && node.children().is_empty(),
        _ => false,
    }
}

/// Whether `c` is an icon: the icon fonts draw from the private-use area,
/// for which a reader has nothing to say. An icon-only button read as a
/// nameless button, and "+  Add file…" began with a stray character.
fn is_icon(c: char) -> bool {
    ('\u{E000}'..='\u{F8FF}').contains(&c)
}

/// `text` without its icons, trimmed; `None` when it has none.
fn strip_icons(text: &str) -> Option<String> {
    text.contains(is_icon).then(|| {
        let kept: String = text.chars().filter(|c| !is_icon(*c)).collect();
        kept.trim().to_string()
    })
}

/// Drop icons from the node's name, description and value. `true` when
/// one of them was nothing but icons.
fn without_icons(node: &mut Node) -> bool {
    // Text runs carry per-character positions, and text fields hold what
    // the person typed: neither is ours to rewrite.
    if matches!(
        node.role(),
        Role::TextRun
            | Role::TextInput
            | Role::MultilineTextInput
            | Role::PasswordInput
            | Role::SearchInput
    ) {
        return false;
    }
    let mut emptied = false;
    macro_rules! clean {
        ($get:ident, $set:ident, $clear:ident) => {
            if let Some(kept) = node.$get().and_then(strip_icons) {
                if kept.is_empty() {
                    node.$clear();
                    emptied = true;
                } else {
                    node.$set(kept);
                }
            }
        };
    }
    clean!(label, set_label, clear_label);
    clean!(description, set_description, clear_description);
    clean!(value, set_value, clear_value);
    emptied
}

/// Turn egui's tree on or off for this frame: on only while the setting
/// allows it and a screen reader is listening. Called before the frame
/// runs, so the tree is in the frame that follows the reader's request.
pub fn sync_egui(ctx: &egui::Context, allowed: bool, bridge: &ScreenReaderBridge) {
    if allowed && bridge.is_listening() {
        ctx.enable_accesskit();
    } else {
        ctx.disable_accesskit();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::accesskit::Action;
    use std::sync::atomic::AtomicUsize;

    fn handler() -> (Handler, Arc<AtomicUsize>) {
        let wakes = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&wakes);
        let handler = Handler {
            shared: Arc::default(),
            wake: Arc::new(move || {
                counter.fetch_add(1, Ordering::Relaxed);
            }),
        };
        (handler, wakes)
    }

    fn click(n: u64) -> ActionRequest {
        ActionRequest {
            action: Action::Click,
            target: NodeId(n),
            data: None,
        }
    }

    #[test]
    fn a_reader_asking_for_the_tree_turns_it_on_and_wakes_the_loop() {
        let (mut h, wakes) = handler();
        assert!(
            h.request_initial_tree().is_none(),
            "the tree comes with a frame"
        );
        assert!(h.shared.listening.load(Ordering::Acquire));
        assert_eq!(wakes.load(Ordering::Relaxed), 1);
        h.deactivate_accessibility();
        assert!(!h.shared.listening.load(Ordering::Acquire));
    }

    #[test]
    fn requests_are_queued_capped_and_woken_for() {
        let (mut h, wakes) = handler();
        for n in 0..(MAX_PENDING_REQUESTS as u64 + 10) {
            h.do_action(click(n));
        }
        let queued = h.shared.requests.lock().unwrap();
        assert_eq!(queued.len(), MAX_PENDING_REQUESTS);
        assert_eq!(queued[0].target, NodeId(0));
        assert_eq!(wakes.load(Ordering::Relaxed), MAX_PENDING_REQUESTS + 10);
    }

    fn node(role: Role, label: &str) -> Node {
        let mut node = Node::new(role);
        node.set_label(label);
        node
    }

    #[test]
    fn the_published_tree_names_the_application_and_its_window() {
        let update = prepared(TreeUpdate {
            nodes: vec![(NodeId(1), Node::new(Role::Window))],
            tree: Some(Tree::new(NodeId(1))),
            focus: NodeId(1),
        });
        let tree = update.tree.as_ref().unwrap();
        assert_eq!(tree.app_name.as_deref(), Some(APP_NAME));
        assert_eq!(tree.toolkit_name.as_deref(), Some("egui"));
        assert_eq!(update.nodes[0].1.label(), Some(APP_NAME));
    }

    #[test]
    fn icons_are_dropped_from_what_is_read() {
        let gear = crate::ui::icons::SETTINGS;
        let plus = crate::ui::icons::ADD;
        let mut label = Node::new(Role::Label);
        label.set_value(format!("{plus}  Add file…"));
        let update = prepared(TreeUpdate {
            nodes: vec![
                (NodeId(2), node(Role::Button, gear)),
                (NodeId(3), node(Role::Button, &format!("{plus}  Add file…"))),
                (NodeId(4), node(Role::Button, "Ctrl+K")),
                (NodeId(5), label),
            ],
            tree: None,
            focus: NodeId(2),
        });
        let [(_, only_icon), (_, icon_and_text), (_, plain), (_, label)] = &update.nodes[..] else {
            panic!("four nodes");
        };
        assert_eq!(only_icon.label(), None, "an icon alone names nothing");
        assert_eq!(icon_and_text.label(), Some("Add file…"));
        assert_eq!(plain.label(), Some("Ctrl+K"));
        assert_eq!(label.value(), Some("Add file…"));
    }

    #[test]
    fn nodes_left_with_nothing_to_say_are_hidden() {
        let mut icon_label = Node::new(Role::Label);
        icon_label.set_value(crate::ui::icons::SETTINGS);
        let mut text_label = Node::new(Role::Label);
        text_label.set_value("Nothing selected");
        let mut with_child = Node::new(Role::Unknown);
        with_child.push_child(NodeId(99));
        let update = prepared(TreeUpdate {
            nodes: vec![
                (NodeId(7), icon_label),
                (NodeId(8), text_label),
                (NodeId(9), Node::new(Role::Unknown)),
                (NodeId(10), with_child),
                (NodeId(11), node(Role::Button, crate::ui::icons::SETTINGS)),
            ],
            tree: None,
            focus: NodeId(8),
        });
        let hidden: Vec<bool> = update.nodes.iter().map(|(_, n)| n.is_hidden()).collect();
        // An icon-only *button* stays: it is a control, and a missing name
        // there is a bug to fix at the button (`ui::accessible`), not to
        // hide.
        assert_eq!(hidden, [true, false, true, false, false]);
    }

    #[test]
    fn focus_on_a_hidden_node_moves_to_the_window() {
        let update = prepared(TreeUpdate {
            nodes: vec![
                (NodeId(1), Node::new(Role::Window)),
                (NodeId(12), Node::new(Role::Unknown)),
            ],
            tree: Some(Tree::new(NodeId(1))),
            focus: NodeId(12),
        });
        assert!(update.nodes[1].1.is_hidden());
        assert_eq!(update.focus, NodeId(1));
    }

    #[test]
    fn with_the_setting_off_the_reader_gets_an_empty_named_window() {
        let update = withheld_tree();
        assert_eq!(update.nodes.len(), 1);
        let (id, window) = &update.nodes[0];
        assert_eq!(*id, WITHHELD_ROOT);
        assert_eq!(window.role(), Role::Window);
        assert_eq!(window.label(), Some(APP_NAME));
        assert!(window.children().is_empty());
        let tree = update.tree.unwrap();
        assert_eq!(
            (tree.root, tree.app_name.as_deref()),
            (WITHHELD_ROOT, Some(APP_NAME))
        );
    }

    #[test]
    fn text_fields_are_left_as_typed() {
        let typed = format!("a{}b", crate::ui::icons::ADD);
        let mut field = Node::new(Role::TextInput);
        field.set_value(typed.clone());
        let update = prepared(TreeUpdate {
            nodes: vec![(NodeId(6), field)],
            tree: None,
            focus: NodeId(6),
        });
        assert_eq!(update.nodes[0].1.value(), Some(typed.as_str()));
    }
}
