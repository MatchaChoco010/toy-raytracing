//! Vulkanのオブジェクトに対するdestroy処理を忘れたりすることをなくすために用意したラッパーライブラリ。
//! Vulkanの各Objectを参照カウンタで管理して、参照がすべて破棄された際に
//! 自動で各種destroy処理を行うようにしたラッパーの構造体の各種Handleが用意されている。
//!
//! 参照カウントの実装には「詳解 Rustアトミック操作とロック ―並行処理実装のための低レイヤプログラミング」の
//! Arcの実装を参考にしている。
//! メモリのOrderingなどは、それに準拠している。
//!
//! 基本的にHandle系の構造体は元のVulkanのオブジェクトのメソッドを引き継いでいる。
//! Vulkanの標準以上の便利メソッドはutilsの中で提供する方針。
#![warn(missing_docs)]

pub mod handles;
pub use handles::*;

pub mod utils;
