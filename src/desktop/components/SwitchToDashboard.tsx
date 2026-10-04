import { useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useProjects } from '../context/ProjectContext';

/**
 * Opens the dashboard whenever the project switches, from the dropdown, the tray or a
 * removed active project. Otherwise the web view reopens the last page the shell was
 * sent to (e.g. Config files), which may not apply to the new project.
 */
export default function SwitchToDashboard() {
  const navigate = useNavigate();
  const { switching } = useProjects();

  useEffect(() => {
    if (switching) navigate('/', { replace: true });
  }, [switching, navigate]);

  return null;
}
