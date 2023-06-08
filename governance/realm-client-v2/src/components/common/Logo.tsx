import React from 'react'

export default function Logo() {
    return (
        <div className='flex gap-2 items-center '>
            <svg width="32" height="32" viewBox="0 0 32 32" fill="none" xmlns="http://www.w3.org/2000/svg">
                <path d="M21.6672 28.1374C21.5218 28.1933 21.4048 28.0155 21.5079 27.8986C24.3046 24.7264 26.0013 20.5613 26.0013 15.9999C26.0013 11.4384 24.3046 7.27326 21.5079 4.10109C21.4048 3.98421 21.5218 3.80636 21.6672 3.86229C26.5419 5.73742 30.0013 10.4646 30.0013 15.9999C30.0013 21.5351 26.5419 26.2623 21.6672 28.1374Z" fill="#00C2FF" />
                <path d="M22.7389 8.35852C22.8039 8.50619 22.5978 8.65459 22.4674 8.55961C21.8598 8.11704 20.7827 7.5 19.5 7.5C17 7.5 13.5 10.5 13.5 16C13.5 21.5 16 24.5 19 24.5C20.6159 24.5 21.8139 23.8734 22.4593 23.4301C22.5923 23.3388 22.8039 23.4938 22.7389 23.6415C21.1225 27.3165 17.8157 30 14 30C7.92487 30 2 23.732 2 16C2 8.26801 7.92487 2 14 2C17.8157 2 21.1225 4.68351 22.7389 8.35852Z" fill="url(#paint0_radial_4622_127300)" />
                <defs>
                    <radialGradient id="paint0_radial_4622_127300" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(19 16) rotate(180) scale(15.1067 20.1422)">
                        <stop offset="0.156146" stopColor="#006585" />
                        <stop offset="1" stopColor="#00C2FF" />
                    </radialGradient>
                </defs>
            </svg>
            <p className='uppercase text-white font-bold'>Realms</p>
        </div>
    )
}
