use std::env;
use std::fs::{self, File};
use std::io::{self, Write, BufReader, Read};
use std::path::PathBuf;
use walkdir::{WalkDir, DirEntry};
use content_inspector::{inspect, ContentType};

fn main() -> io::Result<()> {
    // コマンドライン引数の取得（プログラム名を除く）
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        eprintln!("使い方: folder_to_text <対象ファイルまたはディレクトリのパス> [<対象ファイルまたはディレクトリのパス> ...]");
        std::process::exit(1);
    }

    // 出力ファイルのパス
    let output_file = "output.txt";
    let mut output = File::create(output_file)?;

    // 各引数を処理
    for arg in args {
        let input_path = PathBuf::from(&arg);

        if !input_path.exists() {
            eprintln!("指定されたパスは存在しません: {}", input_path.display());
            continue;
        }

        if input_path.is_dir() {
            // ディレクトリの場合、再帰的に探索
            if let Err(e) = process_directory(&input_path, &mut output) {
                eprintln!("ディレクトリの処理中にエラーが発生しました: {} - {}", input_path.display(), e);
                continue;
            }
        } else if input_path.is_file() {
            // ファイルの場合、単独で処理
            if let Err(e) = process_file(&input_path, &mut output) {
                eprintln!("ファイルの処理中にエラーが発生しました: {} - {}", input_path.display(), e);
                continue;
            }
        } else {
            eprintln!("指定されたパスはファイルでもディレクトリでもありません: {}", input_path.display());
            continue;
        }
    }

    println!("テキストファイルの内容を '{}' に出力しました。", output_file);
    Ok(())
}

/// ディレクトリを再帰的に探索し、テキストファイルを処理する
fn process_directory(dir: &PathBuf, output: &mut File) -> io::Result<()> {
    // ディレクトリ内を再帰的に探索
    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_entry(|e| !is_excluded(e)) // 除外ディレクトリをフィルタ
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // ディレクトリはスキップ
        if path.is_dir() {
            continue;
        }

        // ファイルを処理
        if let Err(e) = process_file(&path.to_path_buf(), output) {
            eprintln!("ファイルの処理中にエラーが発生しました: {} - {}", path.display(), e);
            continue;
        }
    }
    Ok(())
}

/// 単一のファイルを処理する
fn process_file(file_path: &PathBuf, output: &mut File) -> io::Result<()> {
    // ファイルの読み込み
    let file = match File::open(file_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("ファイルを開く際にエラーが発生しました: {} - {}", file_path.display(), e);
            return Ok(()); // エラー発生時はスキップ
        }
    };
    let mut reader = BufReader::new(file);
    let mut buffer = [0; 512];
    let n = match reader.read(&mut buffer) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("ファイルを読み込む際にエラーが発生しました: {} - {}", file_path.display(), e);
            return Ok(()); // エラー発生時はスキップ
        }
    };

    // コンテンツタイプの判定
    let content_type = inspect(&buffer[..n]);

    // テキストファイルのみ処理
    if is_text(content_type) {
        // 相対パスを取得（プログラムの実行ディレクトリからの相対パス）
        let relative_path = match file_path.strip_prefix(&env::current_dir()?) {
            Ok(p) => p,
            Err(_) => file_path.as_path(),
        };

        // ファイル内容の読み込み
        let content = match fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("ファイルを文字列として読み込む際にエラーが発生しました: {} - {}", file_path.display(), e);
                return Ok(()); // エラー発生時はスキップ
            }
        };

        // 出力ファイルに書き込む
        if let Err(e) = writeln!(output, "<{}>", relative_path.display()) {
            eprintln!("出力ファイルへの書き込みに失敗しました: {}", e);
            return Ok(());
        }
        if let Err(e) = writeln!(output, "{}", content) {
            eprintln!("出力ファイルへの書き込みに失敗しました: {}", e);
            return Ok(());
        }
        if let Err(e) = writeln!(output, "</{}>\n", relative_path.display()) {
            eprintln!("出力ファイルへの書き込みに失敗しました: {}", e);
            return Ok(());
        }
    }

    Ok(())
}

/// エントリが除外ディレクトリ（.gitなど）でないかをチェック
fn is_excluded(entry: &DirEntry) -> bool {
    // 除外したいディレクトリ名のリスト
    let excluded_dirs = [".git"];

    entry
        .path()
        .components()
        .any(|comp| {
            // `comp.as_os_str()` を `&str` に変換し、`excluded_dirs` に含まれているかを確認
            comp.as_os_str()
                .to_str()
                .map_or(false, |s| excluded_dirs.contains(&s))
        })
}

/// コンテンツタイプがテキストかどうかを判定
fn is_text(content_type: ContentType) -> bool {
    matches!(
        content_type,
        ContentType::UTF_8
            | ContentType::UTF_8_BOM
            | ContentType::UTF_16LE
            | ContentType::UTF_16BE
    )
}
