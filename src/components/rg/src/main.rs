#![allow(unused)]

mod green;
mod red;
mod text;

use crate::{
    green::GreenNode,
    red::RedNode,
    text::{TextEdit, TextLength, TextPosition, TextRange},
};
use std::{
    cmp::max,
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::Arc,
};

fn main() {
    let content = r#"
      ["rust", "parser",

      [1034,
      "ending"]

      ]
    "#;

    println!("Original source: {}", content);
    let root = RedNode::new_arc(
        Err(PathBuf::from("my_file.txt").into_boxed_path()),
        GreenNode::parse_root(&content),
        TextPosition(0),
    );
    println!("Original tree:");
    root.print(0);

    let edit = TextEdit {
        range: TextRange::new(TextPosition(54), TextLength(1)),
        replace_with: "",
    };

    root.print(0);
    println!("{}", content);
    println!("Changing");
    let (root, content) = root.reparse(content, &edit);
    println!("{}", content);
    root.print(0);

    assert_eq!(content, root.green.flatten());

    /*let (new_root, new_content) =
    green_tree.reparse(&root, content, edit_index, delete_len, insert_text);*/
    //dbg!(new_root, new_content);
}
