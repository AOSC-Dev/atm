message = Message
exit = Exit
cancel = Cancel
proceed = Proceed
error = Error
ok = OK
closed = [closed]
name = Name
date = Date
description = Description
topic_selection = Topic Selection
topic_selection_description = Here below is a list of active update topics available for early adoption.
    Select one or more topic to enroll in update testing, deselect to withdraw and rollback to stable packages.
    Use arrow keys to navigate and use Enter to select/deselect.
topic_selection_closed_topic_warning = Closed/graduated topics detected, ATM will refresh all packages affected by these topics with versions found in the stable repository.

refresh_manifest = Fetching manifest...
refresh_apt = Downloading packages information...
nothing = Nothing to do.
dpkg_error = dpkg returned error: {$status}
no_stable_version = {$count ->
    [one] Notice: there is one package that does not have a stable version to downgrade to.
    *[other] Notice: {$count} packages could not be downgraded to their stable versions.
}
install_count = {$count ->
    [one] one package will be installed
    *[other] {$count} packages will be installed
}
erase_count = {$count ->
    [one] one package will be uninstalled
    *[other] {$count} packages will be uninstalled
}
update_count = {$count ->
    [one] one package will be upgraded or downgraded
    *[other] {$count} packages will be upgraded or downgraded
}
package_path_error = Package path could not be parsed.
#disk_space_decrease = After this operation, {$size} of additional disk space will be used.
#disk_space_increase = After this operation, {$size} of additional disk space will be freed.
details = Details
tx_title = Transaction Details
tx_body = The following operations will be performed:
tx_hold = Kept Back: {$package} (No stable version)
tx_install = Install: {$package} ({$version})
tx_upgrade = Upgrade: {$package} (To {$version})
tx_downgrade = Downgrade: {$package} (To {$version})
tx_erase = Erase: {$package} ({$version})

pk_metered_network = You seem to be on a metered or celluar network.

    ATM may consume a large amount of network data during the transaction.
    Do you still wish to continue?
pk_battery = You seem to be on battery power.

    ATM may deplete the battery rather quickly during the transaction.
    It is recommended to plug in the power supply to prevent sudden power failure.
    Do you still wish to continue?
pk_inhibit_message = ATM transaction is in progress
pk_dbus_error = Failed to connect to D-Bus system bus: {$error}
pk_comm_error_mid_tx = PackageKit daemon unexpectedly disconnected or crashed mid-transaction.

    Your system is likely in an inconsistent state and requires repairing.
    Please quit ATM and run `apt install -f` in your terminal to fix the problem.

    Error message: {$error}
pk_comm_error = Unable to communicate with the PackageKit daemon: {$error}
pk_tx_error = PackageKit daemon reported an error: {$error}
pk_comm_no_response = PackageKit daemon did not return a response.
pk_invalid_id = Package identifier "{$name}" is invalid.

    This is a bug, please report this issue to https://github.com/AOSC-Dev/atm/issues/new.

exe-title = Transaction In-Progress
exe-prepare = Preparing ...
exe-overall = Overall Progress:
exe_download = Downloading {$name}...
exe-install = Installing {$name}...
#exe_verify = [{$curr}/{$total}] Verifying {$name}...
exe_download_file_error = Download failed: {$name}
exe_download_error = Unable to download files
#exe_verify_error = Verification failed: {$name}
#exe_path_error = Filename unknown: {$name}
#exe_batch_error = Failed to download packages

apt_finished = APT configuration updated successfully.
install_error = An error occurred while installing packages: {$error}

#press_enter_to_return = Press Enter to return to the main menu.
press_enter_to_bail = Press Enter to return to quit.

needs-root = Please run me as root!
