# NewNetLoginer-rs

适用于新验证系统的上海科技大学网络自动验证登录器。

## 使用

复制 `.env` 并编辑：

```bash
cp .env.example .env
vim .env
```

运行：

```bash
cargo run
```

注意：除了账密错误，**验证码识别错误**也会导致登陆失败，所以可以多试几次（汗）。
