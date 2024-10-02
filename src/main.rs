use std::io::{self, BufRead, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;
use std::thread;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opt {
    /// Таймаут для подключения
    #[structopt(short = "t", long = "timeout", default_value = "10s")]
    timeout: String,

    /// Хост для подключения
    host: String,

    /// Порт для подключения
    port: u16,
}

fn main() -> io::Result<()> {
    let opt = Opt::from_args();

    // Парсинг таймаута
    let timeout = opt.timeout.trim_end_matches('s').parse::<u64>().unwrap_or(10);
    // Удаляем символ 's' из конца строки и преобразуем в число. 
    // Если преобразование не удалось, используем значение по умолчанию 10 секунд.

    let addr = format!("{}:{}", opt.host, opt.port);

    // Попытка установить соединение с указанным таймаутом
    let socket_addrs = addr.to_socket_addrs()?;
    // Разрешаем имя хоста или IP-адрес в набор сетевых адресов.
    let addr = match socket_addrs.into_iter().next() {
        Some(addr) => addr,
        None => {
            eprintln!("Не удалось получить адрес для {}", addr);
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Не удалось получить адрес"));
        }
    };

    println!("Попытка подключения к {}...", addr);
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(timeout))?;
    println!("Подключение успешно установлено к {}.", addr);

    // Установка сокета в неблокирующий режим
    let _ = stream.set_nonblocking(true);
    // Позволяет читать/писать данные из сокета без блокировки потока выполнения.
    let mut stream_clone = stream.try_clone()?;

    // Поток для чтения из сокета
    thread::spawn(move || {
        let mut buffer = [0; 1024];
        loop {
            match stream_clone.read(&mut buffer) {
                Ok(0) => break, // Соединение закрыто сервером
                Ok(n) => {
                    let data = &buffer[..n];
                    io::stdout().write_all(data).unwrap();
                    io::stdout().flush().unwrap();
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    // Будет блокировка, продолжаем
                }
                Err(e) => {
                    eprintln!("Ошибка чтения из сокета: {}", e);
                    break;
                }
            }
        }
    });
    // Создаем отдельный поток для чтения данных из сокета, чтобы не блокировать основной поток записи./

    // Основной поток для записи в сокет
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        match line {
            Ok(input) => {
                if input.is_empty() {
                    break; // Выход при пустой строке
                }
                if let Err(e) = stream.write_all(input.as_bytes()) {
                    eprintln!("Ошибка записи в сокет: {}", e);
                    break;
                }
            }
            Err(_) => break, // Завершение работы при ошибке чтения
        }
    }
    // Читаем строки из стандартного ввода и записываем их в сокет.

    // Закрытие сокета
    drop(stream);
    // Выход из области видимости сокета приводит к его закрытию.
    println!("Соединение закрыто. Программа завершена.");
    Ok(())
}