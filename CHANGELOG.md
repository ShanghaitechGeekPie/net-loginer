## [0.5.0] - 2024-09-23

- 真静态编译，不再需要手动安装 `onnxruntime`，体积因此大幅增加

## [0.4.2] - 2024-05-16

- 验证码错误时不再重复获取 pushPageId 和 ssid

## [0.4.1] - 2024-05-12

- 将 `image` 替换成 `resize` 和 `zune-jpeg` 以减小包体积

## [0.4.0] - 2024-05-11

- 新增支持多个架构（`x86_64`、`aarch64`）（`Windows`、`Linux`、`macOS`）

## [0.3.1] - 2024-05-10

- 小幅减小模型体积
- 使用 `ureq` 替换 `reqwest`，进一步减小包体积

## [0.3.0] - 2024-05-06

- 使用自定义模型大幅减小包体积

## [0.2.0] - 2024-05-05

- 能够正确地处理验证码错误并不断重试
- 新增用户不存在、密码错误、用户已锁定的错误提示
- 稍微减小了一点包体积

## [0.1.0] - 2024-05-04

- Initial release
- 在 Windows 和 Linux 上通过测试

[0.5.0]: https://github.com/ShanghaitechGeekPie/net-loginer/releases/tag/v0.5.0
[0.4.2]: https://github.com/ShanghaitechGeekPie/net-loginer/releases/tag/v0.4.2
[0.4.1]: https://github.com/ShanghaitechGeekPie/net-loginer/releases/tag/v0.4.1
[0.4.0]: https://github.com/ShanghaitechGeekPie/net-loginer/releases/tag/v0.4.0
[0.3.1]: https://github.com/ShanghaitechGeekPie/net-loginer/releases/tag/v0.3.1
[0.3.0]: https://github.com/ShanghaitechGeekPie/net-loginer/releases/tag/v0.3.0
[0.2.0]: https://github.com/ShanghaitechGeekPie/net-loginer/releases/tag/v0.2.0
[0.1.0]: https://github.com/ShanghaitechGeekPie/net-loginer/releases/tag/v0.1.0
