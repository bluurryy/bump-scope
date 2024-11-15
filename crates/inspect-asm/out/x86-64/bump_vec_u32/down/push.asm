inspect_asm::bump_vec_u32::down::push:
	push rbp
	push rbx
	push rax
	mov rax, qword ptr [rdi + 8]
	cmp qword ptr [rdi + 16], rax
	je .LBB0_1
.LBB0_0:
	mov rcx, qword ptr [rdi]
	mov dword ptr [rcx + 4*rax], esi
	inc rax
	mov qword ptr [rdi + 8], rax
	add rsp, 8
	pop rbx
	pop rbp
	ret
.LBB0_1:
	mov ebp, esi
	mov esi, 1
	mov rbx, rdi
	call bump_scope::mut_bump_vec::MutBumpVec<T,A>::generic_grow_amortized
	mov esi, ebp
	mov rdi, rbx
	mov rax, qword ptr [rbx + 8]
	jmp .LBB0_0
