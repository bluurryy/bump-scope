inspect_asm::alloc_zst::bumpalo:
	push rax
	mov rcx, qword ptr [rdi + 16]
	mov rax, qword ptr [rcx + 32]
	cmp rax, qword ptr [rcx]
	jb .LBB_1
.LBB_2:
	pop rcx
	ret
.LBB_1:
	mov esi, 1
	xor edx, edx
	call qword ptr [rip + bumpalo::Bump::alloc_layout_slow@GOTPCREL]
	test rax, rax
	jne .LBB_2
	call qword ptr [rip + bumpalo::oom@GOTPCREL]