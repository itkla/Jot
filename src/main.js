// import { invoke } from '@tauri-apps/api/core';
// Removed problematic import: import { basename } from '@tauri-apps/api/path';

let currentFilePath = null;
let isModified = false;
let editor;
let titleUpdatePending = false;
let autoSaveTimer = null;
let hasRecoveredContent = false;
let quitRequestUnsubscribe = null;

// simple debounce utility
function debounce(fn, delay) {
    let timer;
    return (...args) => {
        clearTimeout(timer);
        timer = setTimeout(() => fn(...args), delay);
    };
}

// Simple basename function to replace the problematic import
function getBasename(filePath) {
    if (!filePath) return null;
    return filePath.split(/[/\\]/).pop();
}

const invoke = window.__TAURI__.core.invoke;

function debouncedUpdateTitle() {
    if (titleUpdatePending) return;
    
    titleUpdatePending = true;
    requestAnimationFrame(() => {
        const fileName = currentFilePath ? getBasename(currentFilePath) : null;
        invoke('update_title', { 
            filename: fileName, 
            isModified: isModified 
        }).catch(console.error).finally(() => {
            titleUpdatePending = false;
        });
    });
}

// Auto-save functionality
function scheduleAutoSave() {
    if (autoSaveTimer) {
        clearTimeout(autoSaveTimer);
    }
    
    autoSaveTimer = setTimeout(async () => {
        try {
            console.log('Auto-saving content...', editor.value.length, 'characters');
            invoke('auto_save_draft', { content: editor.value }).catch(console.error);
            console.log('Auto-save completed successfully');
        } catch (error) {
            console.error('Auto-save failed:', error);
        }
    }, 1500);
}

// Force auto-save immediately
async function forceAutoSave() {
    if (autoSaveTimer) {
        clearTimeout(autoSaveTimer);
        autoSaveTimer = null;
    }
    
    try {
        console.log('Force auto-saving content...');
        await invoke('auto_save_draft', { content: editor.value });
        console.log('Force auto-save completed');
    } catch (error) {
        console.error('Force auto-save failed:', error);
    }
}

// check for recovery content on startup
async function checkForRecovery() {
    try {
        console.log('Checking for recovery content...');
        const recoveryContent = await invoke('get_recovery_content');
        console.log('Recovery content result:', recoveryContent ? `Found ${recoveryContent.length} characters` : 'None');
        console.log('hasRecoveredContent flag:', hasRecoveredContent);
        
        if (recoveryContent && !hasRecoveredContent) {
            console.log('Recovery content found, prompting user...');
            
            let shouldRecover = false;
            
            try {
                console.log('Attempting to import dialog plugin...');
                const { ask } = await import('@tauri-apps/plugin-dialog');
                console.log('Dialog plugin imported successfully');
                
                console.log('Showing recovery dialog...');
                shouldRecover = await ask(
                    'We found unsaved changes from your previous session. Would you like to recover them?',
                    { title: 'Recovery', kind: 'warning' }
                );
                console.log('Dialog result:', shouldRecover);
                
            } catch (dialogError) {
                console.error('Frontend dialog failed, falling back to backend dialog:', dialogError);
                
                try {
                    shouldRecover = await invoke('show_recovery_dialog');
                    console.log('Backend dialog result:', shouldRecover);
                } catch (backendError) {
                    console.error('Backend dialog also failed:', backendError);
                    shouldRecover = confirm('We found unsaved changes from your previous session. Would you like to recover them?');
                    console.log('Browser confirm result:', shouldRecover);
                }
            }
            
            if (shouldRecover) {
                console.log('User chose to recover content');
                editor.value = recoveryContent;
                isModified = true;
                hasRecoveredContent = true;
                debouncedUpdateTitle();
            } else {
                console.log('User declined recovery, clearing recovery file');
                await invoke('clear_recovery_file');
            }
        } else {
            console.log('No recovery content found or already recovered');
        }
    } catch (error) {
        console.error('Recovery check failed:', error);
    }
}

// handle quit requests
async function handleQuitRequest() {
    console.log('handleQuitRequest called - isModified:', isModified);
    try {
        const shouldQuit = await invoke('handle_quit_request', {
            hasUnsavedChanges: isModified
        });
        
        console.log('handle_quit_request result:', shouldQuit);
        
        if (shouldQuit) {
            console.log('Quitting application...');
            await invoke('exit_app');
        } else {
            console.log('User chose not to quit');
        }
    } catch (error) {
        console.error('Error handling quit request:', error);
    }
}

// editor change handler
function onEditorChange() {
    if (!isModified) {
        isModified = true;
        debouncedUpdateTitle();
    }
    
    // schedule auto-save
    scheduleAutoSave();
    if (editor.value.length > 0 && editor.value.length % 100 === 0) {
        forceAutoSave();
    }
}

// file operations
async function newFile() {
    try {
        await invoke('new_file');
    } catch (error) {
        console.error('Error creating new window:', error);
    }
}

async function openFile() {
    try {
        const result = await invoke('open_file_with_confirmation', {
            hasUnsavedChanges: isModified
        });
        
        if (result) {
            const [content, filePath] = result;
            editor.value = content;
            currentFilePath = filePath;
            isModified = false;
            hasRecoveredContent = false;
            await invoke('clear_recovery_file');
            editor.focus();
        }
    } catch (error) {
        console.error('Error opening file:', error);
    }
}

async function saveFile() {
    try {
        const result = await invoke('save_file', {
            filePath: currentFilePath,
            content: editor.value
        });
        if (result) {
            currentFilePath = result;
            isModified = false;
            hasRecoveredContent = false;
            debouncedUpdateTitle();
        }
    } catch (error) {
        console.error('Error saving file:', error);
    }
}

async function saveAsFile() {
    try {
        const result = await invoke('save_as_file', {
            content: editor.value
        });
        if (result) {
            currentFilePath = result;
            isModified = false;
            hasRecoveredContent = false;
            debouncedUpdateTitle();
        }
    } catch (error) {
        console.error('Error saving file:', error);
    }
}

async function clearDocument() {
    try {
        const shouldClear = await invoke('clear_document_with_confirmation', {
            hasUnsavedChanges: isModified
        });
        
        if (shouldClear) {
            editor.value = '';
            currentFilePath = null;
            isModified = false;
            hasRecoveredContent = false;
            await invoke('clear_recovery_file');
            debouncedUpdateTitle();
            editor.focus();
        }
    } catch (error) {
        console.error('Error clearing document:', error);
    }
}

// keyboard shortcuts
function handleKeyboardShortcuts(e) {
    const modifier = e.ctrlKey || e.metaKey;
    
    if (modifier && e.key === 'n') {
        e.preventDefault();
        if (e.shiftKey) {
            clearDocument();
        } else {
            newFile();
        }
    } else if (modifier && e.key === 'o') {
        e.preventDefault();
        openFile();
    } else if (modifier && e.key === 's') {
        e.preventDefault();
        forceAutoSave();
        if (e.shiftKey) {
            saveAsFile();
        } else {
            saveFile();
        }
    } else if (modifier && e.key === 'q') {
        e.preventDefault();
        handleQuitRequest();
    } else if (modifier && e.shiftKey && e.key === 'A') {
        // Hidden shortcut for testing auto-save: Cmd+Shift+A
        e.preventDefault();
        forceAutoSave();
    }
}

window.addEventListener('beforeunload', () => {
    if (quitRequestUnsubscribe) {
        console.log('Cleaning up quit-requested listener on beforeunload');
        quitRequestUnsubscribe();
        quitRequestUnsubscribe = null;
    }
    
    if (autoSaveTimer) {
        clearTimeout(autoSaveTimer);
        autoSaveTimer = null;
    }
});

document.addEventListener('visibilitychange', () => {
    // Save when user switches away from the app
    if (document.hidden && isModified && editor?.value?.trim()) {
        forceAutoSave();
        console.log('User switched away from the app - saving...');
    }
});

// Save when user focuses away from the editor
let focusOutTimer = null;
async function init() {
    editor = document.getElementById('editor');
    
    if (!editor) {
        console.error('Editor element not found!');
        return;
    }
    if (!window.__TAURI__?.core?.invoke) {
        console.error('Tauri core invoke API not found');
        return;
    }
    
    hasRecoveredContent = false;
    console.log('Initialized app, hasRecoveredContent reset to:', hasRecoveredContent);
    
    if (quitRequestUnsubscribe) {
        console.log('Cleaning up existing quit-requested listener');
        quitRequestUnsubscribe();
        quitRequestUnsubscribe = null;
    }
    editor.addEventListener('input', debounce(onEditorChange, 100), { passive: true });
    
    // Save when user stops interacting with the editor
    editor.addEventListener('blur', () => {
        if (isModified && editor.value.trim()) {
            focusOutTimer = setTimeout(forceAutoSave, 500);
            console.log('User stopped interacting with the editor - saving...');
        }
    });
    
    editor.addEventListener('focus', () => {
        if (focusOutTimer) {
            clearTimeout(focusOutTimer);
            focusOutTimer = null;
            console.log('User focused on the editor - clearing focusOutTimer');
        }
    });
    
    document.addEventListener('keydown', handleKeyboardShortcuts);
    
    try {
        console.log('Setting up quit-requested event listener...');
        const unlistenQuit = await window.__TAURI__.event.listen(
            'quit-requested',
            (event) => {
                console.log('Received quit-requested event:', event);
                handleQuitRequest();
            },
        );
        quitRequestUnsubscribe = unlistenQuit;
        window.addEventListener('beforeunload', () => unlistenQuit());
        console.log('Successfully set up quit-requested event listener');
    } catch (error) {
        console.error('Failed to set up quit-requested event listener:', error);
    }
    
    debouncedUpdateTitle();
    editor.focus();
    
    setTimeout(checkForRecovery, 500);
}

// Initialize when DOM is loaded
document.addEventListener('DOMContentLoaded', init);

