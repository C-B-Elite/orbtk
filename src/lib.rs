#![crate_name = "orbtk"]
#![crate_type = "lib"]
#![deny(warnings)]
#![feature(const_fn)]

extern crate dces;

pub use dces::prelude::*;
use std::any::Any;
use std::cell::RefCell;
use std::sync::Arc;

extern crate cssparser;
extern crate orbclient;
extern crate orbfont;
extern crate orbimage;
#[macro_use]
extern crate lazy_static;

pub use orbclient::color::Color;

pub use cell::CloneCell;
// pub use drawable::*;
// pub use event::Event;
// pub use layouts::*;
pub use structs::*;
pub use theme::{Selector, Theme};
// pub use traits::*;
// pub use tree::*;
// pub use window::{InnerWindow, Window, Application};
// pub use widgets::*;

pub mod cell;
// pub mod drawable;
// pub mod event;
// pub mod layouts;
pub mod structs;
// pub mod traits;
// pub mod window;
// pub mod draw;
pub mod theme;
// pub mod tree;
// pub mod widgets;

struct RenderSystem {
    renderer: RefCell<Box<RenderBackend>>,
}

pub struct Drawable {
    draw_fn: Box<Fn(&Selector)>,
}

impl Drawable {
    pub fn new(draw_fn: Box<Fn(&Selector)>) -> Self {
        Drawable { draw_fn }
    }

    pub fn draw(&self, selector: &Selector) {
        (self.draw_fn)(selector)
    }
}

impl System for RenderSystem {
    fn run(&self, entities: &Vec<Entity>, ecm: &mut EntityComponentManager) {
        self.renderer.borrow_mut().render();
        for entity in entities {
            if let Ok(drawable) = ecm.borrow_component::<Drawable>(*entity) {
                if let Ok(selector) = ecm.borrow_component::<Selector>(*entity) {
                    drawable.draw(selector);
                }
            }

            if let Ok(selector) = ecm.borrow_component::<Selector>(*entity) {
                println!("{:?}", selector);
            } else {
                println!("No {}", entity);
            }
        }
    }
}

pub enum Template {
    Empty,
    Single(Arc<Widget>),
    Mutli(Vec<Arc<Widget>>),
}

pub trait Widget: Any {
    fn template(&self) -> Template {
        Template::Empty
    }
    fn components(&self) -> Vec<ComponentBox> {
        vec![]
    }
}

#[derive(Default)]
pub struct Border {
    child: Option<Arc<Widget>>,
}

impl Border {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn child(&mut self, child: Arc<Widget>) {
        self.child = Some(child);
    }
}

impl Widget for Border {
    fn template(&self) -> Template {
        if let Some(child) = &self.child {
            Template::Single(child.clone())
        } else {
            Template::Empty
        }
    }

    fn components(&self) -> Vec<ComponentBox> {
        vec![ComponentBox::new(Drawable::new(Box::new(
            |_selector: &Selector| println!("Draw border"),
        )))]
    }
}

pub struct Label {
    pub selector: ComponentBox,
}

impl Label {
    pub fn new(selector: Selector) -> Self {
        Label {
            selector: ComponentBox::new(selector),
        }
    }
}

impl Widget for Label {
    fn components(&self) -> Vec<ComponentBox> {
        vec![]
    }
}

pub struct Button;

impl Widget for Button {
    fn template(&self) -> Template {
        Template::Single(Arc::new(Border::new()))
    }

    fn components(&self) -> Vec<ComponentBox> {
        vec![ComponentBox::new(Selector::new(Some("button")))]
    }
}

#[derive(Default)]
pub struct WidgetManager {
    world: World,
}

impl WidgetManager {
    pub fn new(renderer: RefCell<Box<RenderBackend>>) -> Self {
        let mut world = World::new();
        world
            .create_system(RenderSystem { renderer })
            .with_priority(0)
            .with_filter(|comp| {
                for co in comp {
                    if let Some(_) = co.downcast_ref::<Drawable>() {
                        return true;
                    }
                }
                false
            }).build();

        WidgetManager { world }
    }

    pub fn root(&mut self, root: Arc<Widget>) {
        let mut widgets = vec![];
        self.expand(root, &mut widgets);

        for widget in widgets {
            let mut entity_builder = self.world.create_entity();

            for component in widget.components() {
                entity_builder = entity_builder.with_box(component);
            }

            // add bounds
            entity_builder.with(Rect::new(0, 0, 200, 50)).build();
        }
    }

    fn expand(&mut self, widget: Arc<Widget>, widgets: &mut Vec<Arc<Widget>>) {
        match widget.template() {
            Template::Empty => {
                widgets.push(widget);
                return;
            }
            Template::Single(child) => {
                self.expand(child, widgets);
            }
            Template::Mutli(children) => {
                for child in children {
                    self.expand(child, widgets);
                }
            }
        }

        widgets.push(widget);
    }

    pub fn run(&mut self) {
        self.world.apply_filter_and_sort();
        self.world.run();
    }
}

pub struct Application {
    // list of windows
    // theme
    theme: Arc<Theme>,
    windows: Vec<Window>,
}

impl Application {
    pub fn new() -> Application {
        Application {
            theme: Arc::new(Theme::new()),
            windows: vec![],
        }
    }

    pub fn create_window(&mut self) -> WindowBuilder {
        let theme = self.theme.clone();
        WindowBuilder {
            application: self,
            bounds: Rect::default(),
            title: String::from(""),
            theme: theme.clone(),
            root: None,
            renderer: Box::new(OrbitalBackend::new(
                OrbWindow::new_flags(0, 0, 0, 0, "", &[]).unwrap(),
                orbfont::Font::find(None, None, None).ok(),
                theme,
            )),
        }
    }

    pub fn run(&mut self) {
        for window in &mut self.windows {
            window.run();
        }
    }
}

pub struct WindowBuilder<'a> {
    pub application: &'a mut Application,
    pub bounds: Rect,
    pub title: String,
    pub theme: Arc<Theme>,
    pub root: Option<Arc<Widget>>,
    pub renderer: Box<RenderBackend>,
}

impl<'a> WindowBuilder<'a> {
    pub fn with_bounds(mut self, bounds: Rect) -> Self {
        self.bounds = bounds;
        self
    }

    pub fn with_title<S: Into<String>>(mut self, title: S) -> Self {
        self.title = title.into();
        self
    }

    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = Arc::new(theme);
        self
    }

    pub fn with_root<W: Widget>(mut self, root: W) -> Self {
        self.root = Some(Arc::new(root));
        self
    }

    pub fn with_renderer(mut self, renderer: Box<RenderBackend>) -> Self {
        self.renderer = renderer;
        self
    }

    pub fn build(mut self) {
        self.renderer.bounds(&self.bounds);
        let mut widget_manager = WidgetManager::new(RefCell::new(self.renderer));

        if let Some(root) = self.root {
            widget_manager.root(root.clone());
        }

        let theme = self.theme.clone();
        self.application.windows.push(Window {
            widget_manager,
            bounds: self.bounds,
            title: self.title,
            theme,
        })
    }
}

pub struct Window {
    pub widget_manager: WidgetManager,
    pub bounds: Rect,
    pub title: String,
    pub theme: Arc<Theme>,
}

impl Window {
    pub fn run(&mut self) {
        self.widget_manager.run();
    }
}

pub use orbclient::Window as OrbWindow;
use orbclient::{EventOption, Mode, Renderer};
use std::cell::Cell;

pub trait RenderBackend {
    fn render(&mut self);
    fn bounds(&mut self, bounds: &Rect);
}

pub struct OrbitalBackend {
    inner: OrbWindow,
    font: Option<orbfont::Font>,
    theme: Arc<Theme>,
}

impl OrbitalBackend {
    pub fn new(inner: OrbWindow, font: Option<orbfont::Font>, theme: Arc<Theme>) -> OrbitalBackend {
        OrbitalBackend {
            inner: inner,
            font: font,
            theme,
        }
    }
}

impl Renderer for OrbitalBackend {
    fn width(&self) -> u32 {
        self.inner.width()
    }

    fn height(&self) -> u32 {
        self.inner.height()
    }

    fn data(&self) -> &[Color] {
        self.inner.data()
    }

    fn data_mut(&mut self) -> &mut [Color] {
        self.inner.data_mut()
    }

    fn sync(&mut self) -> bool {
        self.inner.sync()
    }

    fn mode(&self) -> &Cell<Mode> {
        &self.inner.mode()
    }

    fn char(&mut self, x: i32, y: i32, c: char, color: Color) {
        if let Some(ref font) = self.font {
            let mut buf = [0; 4];
            font.render(&c.encode_utf8(&mut buf), 16.0)
                .draw(&mut self.inner, x, y, color)
        } else {
            self.inner.char(x, y, c, color);
        }
    }
}

impl Drop for OrbitalBackend {
    fn drop(&mut self) {
        self.inner.sync();
    }
}

impl RenderBackend for OrbitalBackend {
    fn render(&mut self) {
        self.inner
            .set(self.theme.color("background", &"window".into()));
        self.inner
            .rect(250, 200, 80, 80, Color::rgba(100, 100, 100, 100));
        self.inner.sync();

        'events: loop {
            for event in self.inner.events() {
                match event.to_option() {
                    EventOption::Quit(_quit_event) => break 'events,
                    EventOption::Mouse(evt) => println!(
                        "At position {:?} pixel color is : {:?}",
                        (evt.x, evt.y),
                        self.inner.getpixel(evt.x, evt.y)
                    ),
                    event_option => println!("{:?}", event_option),
                }
            }
        }
    }
    fn bounds(&mut self, bounds: &Rect) {
        self.inner.set_pos(bounds.x, bounds.y);
        self.inner.set_size(bounds.width, bounds.height);
    }
}
