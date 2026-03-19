import ProjectDropdown from './ProjectDropdown';

export default function TitleBar() {
  return (
    <div
      data-tauri-drag-region
      className="h-12 flex items-center px-4 bg-paper border-b border-muted select-none shrink-0"
      style={{ paddingLeft: '80px' }} // macOS traffic lights clearance
    >
      <ProjectDropdown />
    </div>
  );
}
