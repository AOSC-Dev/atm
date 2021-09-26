message = 信息
exit = 退出
cancel = 取消
proceed = 继续
error = 错误
ok = 确定
closed = [已关闭]
name = 名称
date = 日期
description = 描述
topic_selection = 选择尝鲜分支
topic_selection_description = 如下是当前可用于测试的尝鲜分支列表。选中一个或多个尝鲜分支即可获得测试用更新，
    反选即可回滚软件包到稳定版本。请使用方向键浏览，并用回车键 (Enter) 选择分支。
topic_selection_closed_topic_warning = 检测到已关闭或已合并的尝鲜分支，ATM 将会把受影响的包回滚到稳定版本。

refresh_manifest = 正在下载分支信息……
refresh_apt = 正在下载软件包信息……
nothing = 无待办事项。
dpkg_error = dpkg 返回错误码：{$status}
no_stable_version = 提示：无法降级 {$count} 个软件包到稳定版本。
install_count = 将安装 {$count} 个软件包
erase_count = 将卸载 {$count} 个软件包
update_count = 将升级或降级 {$count} 个软件包
package_path_error = 无法解析软件包路径。
disk_space_decrease = 该操作将使用 {$size} 存储空间。
disk_space_increase = 该操作将释放 {$size} 存储空间。
details = 详情
tx_title = 操作详情
tx_body = 将进行如下操作：
tx_hold = 保持不变：{$package}（无稳定版本）
tx_install = 安装：{$package}（{$version}）
tx_upgrade = 升级：{$package}（至 {$version}）
tx_downgrade = 降级：{$package}（至 {$version}）
tx_erase = 卸载：{$package}（{$version}）

pk_metered_network = 您似乎正在使用计费网络或移动数据流量。

    ATM 在执行任务时可能会从网络下载大量数据。您确定要继续吗？
pk_battery = 您的电脑目前似乎正在使用电池供电。

    ATM 在执行任务时可能会消耗大量电量，推荐您接入交流电源以防断电导致数据损坏。
    您确定要继续吗？
pk_inhibit_message = ATM 正在点钞
pk_dbus_error = 无法连接到系统 D-Bus 总线：{$error}
pk_comm_error_mid_tx = PackageKit 守护程序连接丢失或在执行任务时突然崩溃。

    您的系统目前可能处于不稳定状态并需要您手动修复。请退出 ATM 并在终端中运行

    `apt install -f`

    以尝试解决问题。

    错误信息：{$error}
pk_comm_error = 无法与 PackageKit 守护程序通信：{$error}
pk_tx_error = PackageKit 守护程序报错：{$error}
pk_comm_no_response = PackageKit 守护程序无响应。
pk_invalid_id = 包名 "{$name}" 无效。

    程序发生了未预期错误，请于 https://github.com/AOSC-Dev/atm/issues/new 报告该问题。

exe-title = 正在执行任务
exe-prepare = 正在准备 ……
exe-overall = 总进度：
exe_download = 正在下载 {$name} ……
exe-install = 正在安装 {$name} ……
exe_download_file_error = 无法下载：{$name}
exe_download_error = 无法下载文件
#exe_verify_error = 校验出错：{$name}
#exe_path_error = 未知文件名：{$name}
#exe_batch_error = 无法下载软件包

apt_finished = APT 配置信息更新成功。
install_error = 安装软件包时发生错误：{$error}

#press_enter_to_return = 请按 Enter 键返回主菜单。
press_enter_to_bail = 请按 Enter 键退出程序。

## CLI messages

needs-root = Please run me as root!
topic-table-hint = Enrolled topics are marked with a `*` character.
fetch-error-fallback = [!] Failed to fetch available topics. Only enrolled topics are shown.

## Authentication messages

await-authentication = Waiting for authentication to finish ...
authentication-failure = Authentication failed: {$reason}
-run-me-as-root-workaround = Your system is not modified.
    Please run ATM as root in your terminal to workaround this issue.
sudo-failure = ATM can not find any privilege escalation facilities in your system.
    { -run-me-as-root-workaround }
headless-sudo-unsupported = ATM does not support headless privilege escalation.
    { -run-me-as-root-workaround }
