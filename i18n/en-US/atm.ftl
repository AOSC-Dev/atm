message = Message
exit = Exit
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
disk_space_decrease = After this operation, {$size} of additional disk space will be used.
disk_space_increase = After this operation, {$size} of additional disk space will be freed.
details = Details
tx_title = Transaction Details
tx_body = The following operations will be performed:
tx_install = Install: {$package} ({$version})
tx_upgrade = Upgrade: {$package} (To {$version})
tx_downgrade = Downgrade: {$package} (To {$version})
tx_erase = Erase: {$package} ({$version})

apt_finished = APT configuration updated successfully.
